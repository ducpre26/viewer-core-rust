# Lệnh phát triển Viewer Core Rust

## Docker thử nghiệm (Windows/Linux host → Linux amd64 image)

Theo yêu cầu 2026-10-28, bổ sung các script đóng gói và kiểm tra Git metadata.
Runbook và cấu hình volume: [deploy/docker](../deploy/docker/README.md).

| Lệnh | Mục đích |
|---|---|
| `python tools/test_facade_metadata.py` | Kiểm tra truyền Git revision/dirty khi build không có .git và giữ fallback local; không chạy Maven/JVM. |
| `.\scripts\build-docker.ps1 -Image viewer-core-rust:test` | Windows: build/load image linux/amd64 bằng Docker Desktop Linux engine. |
| `bash scripts/build-docker.sh --image viewer-core-rust:test` | Linux: cùng build/load image qua Buildx. |
| `docker compose -f deploy/docker/compose.yaml config --quiet` | Kiểm tra Compose bằng CLI thật, không chạy app. |

Sau khi copy config.example thành config và điền DB/admin, chạy các lệnh
up/health/readiness trong runbook. Các bài DB/JNI/codec phải chạy trên image
thật trước khi xác nhận Docker runtime đạt; syntax/mock không thay kết quả đó.

Chạy các lệnh dưới đây từ thư mục gốc repo `1pacs-viewer-core-rust`. Toolchain Rust
được ghim trong `rust-toolchain.toml`; dependency được khóa trong `Cargo.lock`.

## Cấu hình ứng dụng

Theo [contract TOML](SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#toml-config),
DB/auth/session/JNI dùng chung registry của Summer với HTTP/logger. Giữ nguyên
`config/app.toml`, overlay theo profile, interpolation env và RUST_LOG override.
Không thêm `--config`; module không tự đọc env cho settings ứng dụng.

## 1. Chuẩn bị môi trường

| Lệnh | Mục đích |
|---|---|
| `rustup show` | Xem toolchain và target Rust đang được chọn. |
| `cargo --version` | Kiểm tra Cargo đã sẵn sàng. |

Cần có Python 3 để chạy smoke test. Trên Windows cần MSVC C++ Build Tools; trên
Linux x86_64 cần trình biên dịch C/C++ và linker.

PostgreSQL bắt buộc. Sao chép `config/app-dev.example.toml` thành
`config/app-dev.toml`, sửa `[database]` và Basic admin, rồi chạy từ root repo.
Không cần khai báo từng `$env:VIEWER_*`; vẫn có thể dùng env trong placeholder
TOML theo Summer. File local có mật khẩu được gitignore. Migration tự chạy
trước HTTP; lỗi cấu hình/kết nối làm startup thất bại.

## 2. Chạy khi phát triển

| Lệnh | Mục đích |
|---|---|
| `cargo run --locked -p viewer-app` | Summer đọc app.toml + overlay theo profile; chạy debug và dừng bằng Ctrl+C. |
| `cargo check --workspace --locked` | Kiểm tra biên dịch nhanh sau khi sửa code, không tạo bản chạy hoàn chỉnh. |
| `cargo fmt --all` | Tự định dạng mã Rust. |

Chọn cổng bằng `web.port` trong file TOML, ví dụ `8889` nếu Java đang dùng
8888. Profile mặc định dev; SUMMER_ENV và các placeholder env trong TOML vẫn
được Summer xử lý như trước.

Kiểm tra thủ công khi ứng dụng đang chạy (thay `8888` bằng cổng đã chọn):

| Lệnh | Mục đích |
|---|---|
| `curl http://127.0.0.1:8888/ws/rest/health` | Kiểm tra tiến trình còn sống. |
| `curl http://127.0.0.1:8888/ws/rest/ready` | Kiểm tra readiness của các thành phần đã triển khai. |
| `curl http://127.0.0.1:8888/ws/rest/docs/openapi.json` | Xem OpenAPI của các route hạ tầng. |

## 3. Kiểm tra trước khi hoàn tất thay đổi

| Lệnh | Mục đích |
|---|---|
| `cargo fmt --all -- --check` | Báo lỗi nếu mã chưa đúng định dạng. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Kiểm tra lint và coi cảnh báo là lỗi. |
| `cargo test --workspace --locked` | Chạy test không cần DB; test PostgreSQL bị bỏ qua có ghi rõ. |
| `cargo test -p viewer-app --locked postgres_migrations_transaction_readiness_and_close -- --ignored` | Chạy PostgreSQL thật: migration/transaction/readiness, auth/session HTTP/cache (Step 3a), admin CRUD/mapping, unique/FK/cascade/rollback và cache credential (Step 4), config admin/session CRUD/permission/constraint/cache và ghi đồng thời (Step 5), tạo phiên/link/expiry/snapshot quyền và PATH_PREFIX dùng chung cache (Step 8), lifecycle/chia sẻ/cleanup/URL public và lỗi DB (Step 9), đóng pool. Cần DB riêng cho test. |
| `cargo build --workspace --locked` | Build debug cho toàn workspace. |
| `python tools/smoke_app.py` | Chạy ứng dụng debug thật để kiểm tra HTTP, cấu hình, logging và dừng sạch; Step 8 thêm tạo link/session qua TCP cho cả hai API và dùng session vừa tạo để truy cập config; Step 9 kiểm tra GET, chia sẻ DB và DELETE qua TCP. |
| `cargo test -p viewer-app --locked postgres_migrations_transaction_readiness_and_close_wado -- --ignored` | Step 11: fixture Java, session scope/share-key/cache/DB fallback, stream 16 MiB, backpressure/disconnect, truncated body, timeout và lỗi kết nối. Dùng DB theo TOML/profile hiện tại; chỉ thêm rồi xóa SESSION có UUID riêng của test. |
| `python tools/smoke_wado.py` | Step 11: binary debug thật và Postman store cục bộ qua TCP; dùng DB cấu hình hiện tại, kiểm tra WADO-RS/WADO-URI, upstream error, header/payload, dọn session cache và shutdown. Cần Postman CLI >=1.70; tự chạy/dừng simulation, không đăng nhập hoặc publish. |

Smoke test dùng cổng localhost trống và thư mục config TOML tạm cho từng
subprocess; chạy dev/prod qua profile loader thật. Thêm kiểm tra interpolation,
RUST_LOG override và việc module không đọc env ngoài TOML. Không cần
khởi động ứng dụng bằng tay trước khi chạy test này.

## 4. Kiểm tra bản phát hành

| Lệnh | Mục đích |
|---|---|
| `cargo build --workspace --release --locked` | Tạo bản tối ưu hóa để phát hành. |
| `python tools/smoke_app.py --release` | Kiểm tra hành vi runtime của bản release. |

CI tại `.github/workflows/rust.yml` chạy format, clippy, test, release build và
release smoke test trên Windows và Ubuntu. Kết quả CI chỉ có sau khi workflow
thực sự chạy; file workflow tự nó không chứng minh Linux đã pass.

## 5. Build và kiểm tra Java facade (Step 3)

Cần Python 3, Maven 3.9 và JDK 25. Truyền đường dẫn JDK rõ ràng để Maven và
smoke test dùng cùng runtime. Ví dụ Windows local:

```powershell
python tools/facade.py build --java-home 'C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot'
python tools/facade.py verify --java-home 'C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot'
```

Linux dùng cùng lệnh với đường dẫn JDK tại máy. Nếu cache Maven mặc định không
truy cập được, thêm `--maven-repo <directory>` vào lệnh build; trên máy phát triển
hiện tại dùng `--maven-repo 'C:\Users\Admin\.m2\repository'`.

- `build`: Maven clean/package, tạo thư mục dependency, kiểm tra hai method từ
  JAR thật và ghi manifest/checksum. Đây là kiểm tra packaging Java, chưa phải JNI.
- `verify`: kiểm tra manifest, inventory/checksum JAR rồi chạy lại packaged smoke.
- Artifact: `java/dicom-facade/target/distribution/`; từ Step 12 chứa dependency DICOM/JavaCV/FFmpeg của baseline.
- Log: `target/step-03-logs/`; mỗi lần chạy ghi đè log của công đoạn tương ứng.

Theo dõi build trực tiếp từ terminal riêng:

```powershell
Get-Content target/step-03-logs/build-facade.log -Wait -Tail 50
```

Chi tiết contract và giới hạn kiểm chứng tại [facade README](../java/dicom-facade/README.md).

## 6. Kiểm tra Rust/JNI và ứng dụng (Step 3)

Sau khi build facade, chạy với JDK 25 explicit:

```powershell
python tools/smoke_jni.py --java-home 'C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot'
python tools/smoke_jni.py --java-home 'C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot' --app
python tools/smoke_jni.py --java-home 'C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot' --release --app
```

Lệnh đầu kiểm tra adapter bằng JVM thật trong subprocess riêng, không cần DB.
`--app` cần DB test riêng trong `config/app-dev.toml` và binary debug đã build;
runner dùng TOML fixture cho JNI/subprocess; thêm `--release`
để kiểm tra binary release. Script chọn giới hạn memory/timeout thử nghiệm rõ ràng,
luôn bật `-Xcheck:jni`, kiểm tra readiness 200/503, HTTP dev/prod, shutdown và lỗi
JVM fatal. Cấu hình chạy app thủ công nằm trong [README adapter](../crates/dicom-jvm/README.md).

Không chạy `--include-ignored` toàn workspace khi chỉ có DB: các test JNI cần thêm
runtime và packaged facade. Chạy từng lệnh ở trên để chuẩn bị đúng môi trường.
`smoke_app.py` luôn kiểm tra JNI tắt; `smoke_jni.py --app` kiểm tra JNI bật.

Theo dõi log suite từ terminal riêng:

```powershell
Get-Content target/step-03-logs/jni-suite.log -Wait -Tail 50
```

CI Windows/Ubuntu build facade bằng Temurin 25, chạy các kiểm tra Rust và cả hai
chế độ JNI; log và manifest được lưu thành artifact CI. Build Java thực tế được
ghi trong manifest; đây chưa phải lựa chọn runtime production đã chốt.

## 7. Xem thay đổi trước khi commit

| Lệnh | Mục đích |
|---|---|
| `git status --short` | Xem file đã thêm hoặc sửa. |
| `git diff --check` | Tìm lỗi khoảng trắng trong các file đã theo dõi. |
| `git diff` | Đọc lại các thay đổi đã theo dõi trước khi commit. |

Các lệnh trên kiểm tra nền tảng HTTP và persistence. Với từng nhiệm vụ, chọn lệnh phù hợp
và đối chiếu kết quả với tiêu chí trong kế hoạch step; không cần chạy toàn bộ lệnh sau
mỗi thay đổi nhỏ. Trước khi kết thúc step, chạy đủ các kiểm tra cần thiết cho phạm vi
step và phần bị ảnh hưởng; test bỏ qua hoặc chưa chạy không được coi là đạt.

Khi có module mới, bổ sung test tương ứng và kiểm tra xem các lệnh hiện có đã bao phủ
tiêu chí của step chưa. Nếu cần lệnh mới, báo người quản lý về mục đích và kết quả cần
kiểm chứng để thống nhất bổ sung vào tài liệu này. Trước khi commit, báo kết quả và
hỏi lại người dùng; chỉ commit sau khi được xác nhận.

## 8. Download DICOM/JPEG/MP4 (Step 12)

Mục đích của các lệnh mới: thu baseline download riêng, kiểm chứng codec/output
độc lập và HTTP/ZIP/auth/lease; các lệnh proxy WADO không thay thế các kiểm tra này.

```powershell
# Baseline nguồn Java ở repo bên cạnh đã build; Windows local JDK hiện tại.
python tools/baseline_download.py
# Tái tạo golden MP4 từ Java gốc với input.dcm do DownloadSmoke sinh:
python tools/baseline_download.py --mp4-directory '<codec output directory>'
# Build facade như mục 5, sau đó:
python tools/smoke_download.py --java-home 'C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot' --app
# Công cụ độc lập, chỉ cài vào target; không sửa Python hệ thống:
python -m pip install --target target/step-12-tools -r tools/download-test-requirements.txt
python tools/smoke_download.py --java-home 'C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot' --app --independent
# Theo yêu cầu 2026-10-28: lỗi store, hủy/deadline FFmpeg thật và thời điểm response.
# Cần Postman CLI có simulation; imageio_ffmpeg ở target/step-12-tools.
python tools/test_download_failures.py --java-home 'C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot'
```

Runner lỗi dùng backend debug và facade production đã build, PostgreSQL từ TOML
hiện tại, session chỉ trong cache. Tạo DICOM synthetic 384 frame 512×512 khoảng
96 MiB và encode MP4 thật; không gọi PACS thật. Tự khởi động/dừng simulation trong
`postman/simulations/download-failures.sim.yaml`, không lưu history lên dịch vụ.
Các lỗi TCP chạy trực tiếp với listener của cùng mock vì gateway simulation
không giữ nguyên tình huống socket bị cắt. Có kiểm tra file output bị truncate
sau headers, chỉ tác động thư mục job của runner; không phải bài kiểm tra đĩa đầy.
Log/MP4/results ở `target/step-12-error-logs/`; fixture ở
`target/step-12-error-fixtures/`. `--transport-only` chỉ chạy phần lỗi HTTP, không
được tính là pass cancellation/deadline FFmpeg. Tham số thử nghiệm không chốt P1–P4.

Sau refactor 2026-11-03, store chậm headers/body vẫn trả 200 khi hoàn tất, không
còn 504 do deadline download. Case 504 còn lại kiểm tra deadline JNI dùng chung.
DICOM download vẫn hoạt động khi JNI bận/quá deadline. Runner không cấu hình
quota/TTL; workspace tests kiểm tra trực tiếp cleanup bỏ qua file active.

`--app` dùng PostgreSQL theo TOML/profile hiện tại. Session chỉ tạo trong cache
test, không insert session hoặc thay CONFIG; upstream là HTTP server localhost.
HTTP suite dùng video double trong JAR dưới target, metadata chạy code thật;
JPEG được phục vụ bằng ảnh synthetic hợp lệ tại endpoint rendered của store giả.
Suite kiểm tra frame 1-based, single/multi-frame/ZIP, sai MIME/status/body/truncated,
đầu vào sai định dạng và JPEG instance khi JNI tắt. Runner luôn
chạy MP4 qua facade Java và qua Rust/JNI thật; `--independent` giải mã cả hai bằng
FFmpeg riêng và so pixel. DICOM không transcode; JPEG lấy frame từ WADO-RS rendered
với Accept image/jpeg và quality=90. Rust không dùng Orthanc transcode/native, khác Java.

Log tại `target/step-12-logs/`: fixture, metadata/video, JNI video, HTTP contract,
decoded-output JSON và peak RSS nếu cài psutil. Peak RSS chỉ là workload synthetic
nhỏ được lấy mẫu mỗi 20 ms; chưa thay bài đo series lớn/production P2. Bộ kiểm tra
queue/deadline/exception/reference và cancellation nằm trong lệnh `smoke_jni.py`
ở mục 6, case `download`. Kiểm tra TTL/stream/ZIP64 và transport vượt giới hạn
synthetic cũ nằm trong workspace tests.

Không cần section `[download]`; chỉ `[video].fps` tùy chọn, mặc định 1.
`DOWNLOAD_PATH_FORMAT` vẫn ở DB, có fallback. Khi nâng cấp bỏ section cũ và
chuyển `download.video_fps` sang `[video].fps` nếu cần giữ FPS tùy chỉnh.
File tạm nằm trong vùng `pacs-viewer-core-downloads` của thư mục tạm hệ thống;
lease/lock giữ file active; dọn file mồ côi là chi tiết nội bộ. Không có quota
byte/job/frame/pixel/entry hoặc semaphore download. `jni.timeout_ms` dùng chung
áp dụng từng lời gọi JNI, không phải deadline toàn request download.
Facade API 2 thay chữ ký JNI: build/deploy distribution cùng binary Rust.
JNI cần cho metadata study/series và MP4; DICOM/JPEG instance không cần JVM. File
được chuẩn bị trước khi HTTP stream đọc từng chunk 64 KiB, không buffer toàn bộ
output trong RAM. Xem các giới hạn parity tại [contract](baseline/step-12-contract.md).

Nếu ổ chứa TEMP hệ thống thiếu dung lượng khi build/test, có thể đặt TEMP/TMP
cho riêng tiến trình kiểm tra sang một thư mục target đã tạo trên ổ workspace.
Đây là thư mục công cụ, không thay contract cấu hình DB/auth/session/JNI bằng TOML.


### Kiểm chứng với Orthanc thật trong STORE.md

Theo yêu cầu người dùng 2026-10-27, được phép truy vấn và lưu metadata cục bộ.
Runner chỉ GET tới Orthanc; POST/DELETE session chỉ ở ứng dụng Rust thử nghiệm
trên localhost. Không thay config ứng dụng đang dùng hoặc source repo Java gốc.

```powershell
python tools/verify_live_download.py --samples 3 --java-home 'C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot'
```

Cần binary debug mới, công cụ độc lập ở `target/step-12-tools` và Postman CLI.
Runner lấy URL/auth/study từ STORE.md, không hardcode credential trong source,
không gửi sự kiện Postman lên API Catalog (`--no-report-events`). Metadata/media
thật và kết quả chỉ lưu tại `target/step-12-logs/live-store/` đã gitignore.
DICOM/JPEG instance chạy khi JNI tắt; nếu mẫu đã bao phủ toàn bộ một study có một
series, `--java-home` kiểm tra thêm ZIP DICOM/JPEG study/series bằng metadata Java thật.
DICOM và JPEG được so byte với nguồn; pydicom đọc file và Pillow
đọc frame encapsulated JPEG Baseline khi transfer syntax tương ứng. JPEG server
được giải mã riêng và output JPEG qua Rust được so byte/giải mã lại.
Sai MIME của WADO-URI được ghi riêng; Rust dùng
WADO-RS rendered, yêu cầu HTTP 200, Content-Type image/jpeg, SOI/EOI và kích thước
JPEG hợp lệ. Store phải hỗ trợ `/frames/{1-based}/rendered`; không fallback
sang codec local hoặc trả PNG/DICOM dưới nhãn JPEG.


## 9. Key image (Step 13)

Mục đích: thu hành vi Java gốc rồi kiểm chứng ingest/pending/DICOM, đọc local,
retry/STOW/cleanup qua Rust/JNI/PostgreSQL thật và kho Postman synthetic cục bộ.
Không gửi key image lên PACS thật. Cần PostgreSQL test riêng với bảng
`STUDY_KEY_IMAGE` rỗng; suite từ chối dispatch nếu đã có record trước khi chạy.
Không dùng database vận hành cho bài này.

```powershell
python tools/baseline_key_image.py --java-home 'C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot'
# Build facade bằng lệnh mục 5; cài pydicom/Pillow vào target theo mục 8 nếu chưa có.
python tools/smoke_key_image.py --java-home 'C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot'
```

Baseline compile service Java gốc vào target, dùng repository/session double,
không sửa repo Java; chưa thay cho kiểm chứng Spring container/DB Java thật.
Runner smoke khởi động/dừng Postman simulation không ghi history, chạy test
`postgres_key_image_contract` với JAR production và `-Xcheck:jni`, so DICOM bằng
pydicom/Pillow độc lập, rồi build/chạy binary qua TCP. Bài TCP kiểm tra ingest,
WADO local và pending được xuất/cleanup sau restart bật scheduler. Session/cache,
DB record và file đều dùng dữ liệu synthetic; config fixture không sửa TOML local.

Kết quả ở `target/step-13-logs/`: `java-baseline.log`, `contract.log`,
`simulation.ndjson`, `independent-output.json`, `tcp-result.json` và log binary.
Fixture có version control: `docs/baseline/step-13-java.json`.
Queue/deadline/cancellation/lease/reference của operation key image nằm trong
case `key-image` của `tools/smoke_jni.py` (mục 6); đây là fixture Java riêng để
điều khiển thời điểm call thật, không thay corpus DICOM production trong smoke.
Các tham số JVM trong runner là cấu hình thử nghiệm, không chốt P1–P4 production.

### Step 13 với store QA thật

Theo yêu cầu người dùng 2026-11-06, runner sau đọc `docs/STORE.md`, GET study
tham chiếu và **ghi hai study synthetic mới lên QA**, giữ lại để review.
Không chạy nếu không muốn tạo dữ liệu test trên QA. Cần binary/facade đã build,
pydicom/Pillow tại target/step-12-tools và psql với quyền CREATE/DROP DATABASE.
Runner tạo DB PostgreSQL riêng, dọn DB/config chứa credential sau khi chạy;
không bật scheduler trên database hiện có.

```powershell
python tools/verify_live_key_image.py --java-home 'C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot'
```

Kiểm tra ingest/pending/local metadata, restart scheduler, QA WADO readback,
UID/transfer syntax/pixel bằng công cụ độc lập và cleanup pending/file.
Raw local frame hiện là giới hạn Java, được ghi riêng. Kết quả trong
`target/step-13-logs/live-qa/result.json`; exit 1 nếu toàn flow chưa đạt.
Lượt 2026-11-06 chưa đạt: metadata.json gửi dưới nhãn DICOM khiến QA trả 400,
dù DICOM đã được nhận. Chi tiết và đối chứng ở Step 13.

Có thể chọn store và thư mục bằng chứng riêng, không sửa file store cũ:

```powershell
python tools/verify_live_key_image.py --store-file docs/STORE_2.md --output target/step-13-logs/live-qa-store-2 --java-home 'C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot'
```

Preflight phải trả JSON array QIDO trước khi tạo database hoặc gửi STOW.
Lượt STORE_2 ngày 2026-11-06 dừng ở preflight do HTTP 200/text/html (SPAX UI),
chưa kiểm chứng STOW hoặc readback trên store này.

Lượt STORE_2 cập nhật cuối ngày 2026-11-06 dùng Bearer auth đã đạt bằng kiểm tra
scheduler và delayed readback bổ sung; evidence ở
`target/step-13-logs/live-qa-store-2-attempt-3/`. Có thể dùng `--output` để giữ
bằng chứng riêng từng lượt. Runner nhận `auth Bearer ...`, `auth Basic ...`
hoặc dạng user/password cũ. Đã bổ sung chờ metadata 30 giây và fallback Accept
application/dicom khi store trả 406; chưa chạy lại nguyên runner sau bổ sung.

Theo yêu cầu người dùng, có thể thêm hai key image synthetic vào study được
ghi trong file, trong một series mới, bằng `--reference-study`. Mode này lấy
patient/study metadata của instance gốc, kiểm tra các series cũ được giữ nguyên
và không xóa dữ liệu trên store. Cần study có source series đọc được; không
tự tạo study mới với thông tin bệnh nhân giả khi không tìm thấy study gốc.

```powershell
python tools/verify_live_key_image.py --store-file docs/STORE_2.md --reference-study --output target/step-13-logs/store-2-reference-study --java-home 'C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot'
```

Đối chứng gửi thẳng store, không gọi Rust/JVM/DB, dùng lại hai DICOM/SOP UID
từ `target/step-13-logs/store-2-reference-stow` và STOW URL/auth trong STORE_2.md:

```powershell
python tools/send_key_image_direct.py
```

Chỉ gửi DICOM, không metadata.json; ghi status/body STOW, poll metadata 30 giây
và GET DICOM vào `target/step-13-logs/direct-store-2`. Không xóa remote data.
Exit 0 là phép đo chạy xong, cần đọc status/body và readbackVerified trong
result.json; không tự tính HTTP 200 là kiểm chứng lưu ảnh thành công.


## 10. Kiểm tra DICOM Print (Step 17)

Build và verify facade bằng mục 5 trước khi chạy. Các test chỉ tạo máy in,
phiên và ảnh synthetic riêng, sau đó xóa bằng API.

```powershell
python -m pip install --target target/step-17-tools -r tools/print-test-requirements.txt
python tools/smoke_print.py --java-home 'C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot'
# Khi có target/classes của Java gốc để đối chiếu chuyển đổi raster PNG:
python tools/smoke_print.py --java-home 'C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot' --java-baseline '../1pacs-viewer-core/target/classes'
```

Runner dùng cấu hình PostgreSQL từ TOML, binary Rust, JVM/facade production và
Print SCP pynetdicom cục bộ qua TCP. Kiểm tra CRUD, validation, xác thực phiên,
chiều Calling/Called AET, grayscale, hành vi màu PNG của Java, DIMSE warning,
failure, reject, offline và timeout sau N-ACTION. Kết quả và log nằm trong
`target/step-17-logs/`. Fixture raster ở `docs/baseline/step-17-java.json`.

Runner còn yêu cầu 12 giao dịch in chạm N-ACTION đồng thời trên simulator,
để xác minh khả năng nhận giao dịch độc lập. Upload 9 MiB được xử lý và 11 MiB
bị từ chối theo giới hạn toàn request 10 MiB mặc định của Java. Giới hạn từng
file 100 MiB được áp dụng riêng theo cấu hình Java.

`smoke_jni.py` tại mục 6 có thêm case `print-worker` để kiểm tra in đồng thời,
hủy trước khi gọi Java, giữ file đến completion, timeout, clear exception
và các giao dịch khác tiếp tục chạy khi một giao dịch đang chờ máy in. Hồi quy DB dùng
lệnh PostgreSQL ở mục 3. Những bài này sử dụng simulator; kiểm tra bản in vật lý
cần máy in thực tế.
