# Step 12 - UC-005 Tải nội dung DICOM

## Tính năng

Download DICOM/JPEG theo study, series, instance; MP4 theo series.
Rust xử lý HTTP, ZIP và vòng đời file tạm. Java qua JNI xử lý metadata,
tên đường dẫn ZIP và encode MP4 bằng JavaCV/FFmpeg theo quyết định T14.

**Khác Java:** Rust không dùng Orthanc transcode/native; DICOM giữ nguyên byte,
JPEG lấy từ WADO-RS rendered của store. Repo Java gốc không đổi.

Contract cấu hình hiện hành: download luôn hoạt động, không có `[download]`
hoặc quota/deadline/semaphore download riêng. Chỉ giữ `[video].fps` tùy chọn,
mặc định 1; `DOWNLOAD_PATH_FORMAT` ở DB và fallback như Java. Queue/deadline JNI
dùng chung áp dụng từng lời gọi; lease/lock và cleanup quản lý vòng đời file.
Facade API 2 phải triển khai cùng binary Rust. Chi tiết tại phần refactor bên dưới.

## Nguồn Java

- [DownloadAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/file/rest/DownloadAPI.java)
- [DownloadServiceImpl.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/file/service/DownloadServiceImpl.java)
- [UC-005](../../../1pacs-viewer-core/docs/use_cases/UC-005-tai-noi-dung-dicom.md)
- [Contract và khác biệt đã chốt](../baseline/step-12-contract.md), [baseline 24 case](../baseline/step-12-java.json)

## Việc cần làm

- [x] Facade metadata/video; JNI queue, deadline và ownership cho video.
- [x] Đối chiếu Java, revision, fixture và expected result tại contract.
- [x] Tái sử dụng SESSION/auth và CONFIG `DOWNLOAD_PATH_FORMAT`; không thêm schema/migration.
- [x] Ba route và phạm vi DICOM/JPEG/MP4; kiểm chứng output theo bảng bằng chứng bên dưới.
- [x] Từ chối kiểu tải không hỗ trợ, giữ 415/errorCode theo baseline.
- [x] ZIP structure/naming, chống path không an toàn/tên trùng; HTTP stream file 64 KiB.
- [x] DICOM nguyên byte; JPEG WADO-RS rendered, frame 1-based, quality=90.
- [x] T14: giữ Java FFmpeg binding cho MP4.
- [x] Cancellation giữ file/lease đến completion; file lock bảo vệ trước cleanup.

## Kiểm tra hoàn thành

- [ ] Corpus JNI/codec đủ phạm vi; queue/deadline/cancellation/reference đã có test, corpus rộng còn hoãn.
- [ ] API/output và tác động DB/file khớp baseline trong phạm vi đã chốt; còn xác nhận lỗi với dữ liệu store thật.
- [ ] UC-005 main flow, A1..A3, BR-001..BR-003; golden ZIP/JPEG/MP4 đã có, disk-full và memory peak còn hoãn.
- [x] Cancellation/disconnect khi FFmpeg thật đang encode: lease giữ file tới completion, cleanup bỏ qua job active và dọn sau khi hết owner. Input synthetic, chưa xác nhận với dữ liệu STORE.md.
- [ ] Output mở được bằng tool độc lập đã đạt trên mẫu hiện có; còn corpus codec rộng và kiểm chứng tài nguyên thực tế (đĩa đầy, memory peak/soak, series lớn).

## Tiến độ sau review — 2026-10-28

Đây là lịch sử trước refactor; yêu cầu download limits/P1–P4 và mặc định tắt
bên dưới đã được thay bằng contract rút gọn 2026-11-03 ở cuối tài liệu.

- Trạng thái: `in_progress`. Người dùng xác nhận commit phần đã làm ngày
  2026-10-28; các mục chưa kiểm chứng bên dưới vẫn giữ nguyên.
- Đã triển khai: routes, auth/session, metadata/path, ZIP, JPEG từ store, MP4 qua JNI,
  typed TOML, quota, semaphore, deadline, lease/TTL và file response có Content-Length.
- Đã sửa qua test lỗi: store trả non-200 bị chuyển thành DICOM attachment 200;
  hiện trả lỗi đã lược bỏ chi tiết nguồn trước khi gửi file.
- Cần xác nhận tiếp với STORE.md: MP4 dùng dữ liệu thật, hủy khi đang encode;
  lỗi tải và thời điểm response qua proxy cục bộ chủ động ngắt/chặn nguồn thật.
  Chưa chạy các bài này; 26 bài giả lập không thay bằng chứng store thật.
- Ghi để test sau theo yêu cầu: corpus transfer syntax/color/compressed rộng,
  đĩa thật đầy, series lớn/memory peak/soak, release Windows và Linux/CI.
- Còn quyết định cấu hình production P1–P4; download mặc định tắt, giới hạn trong
  `config/download.example.toml` chỉ dành cho thử nghiệm.
- Lượt review này chỉ chỉnh tài liệu/tiến độ, không chạy thêm test runtime.

## Bằng chứng lịch sử trước refactor — 2026-10-28

Bảng này ghi kết quả phiên bản cũ, gồm permit/quota đã bỏ. Kết quả kiểm chứng
contract hiện hành nằm tại mục **Tiến độ refactor — 2026-11-03** bên dưới;
không dùng bằng chứng cũ để xác nhận hành vi quota/deadline hiện tại.

| Kiểm tra | Nguồn dữ liệu / môi trường | Kết quả và giới hạn |
|---|---|---|
| DICOM/JPEG instance; ZIP study/series | Store thật trong STORE.md: 1 series, 2 instance | Byte khớp nguồn; JPEG giải mã được, DICOM giữ JPEG Baseline. Instance chạy JNI tắt; ZIP dùng Java metadata thật |
| MP4 đối chiếu Java | Synthetic 2 frame 16×16; JavaCV trực tiếp và Rust/JNI thật | FFmpeg độc lập giải mã, pixel khớp Java gốc; chưa đại diện corpus rộng |
| Route/header/naming/auth/ZIP | 24 case Java baseline, HTTP store giả; video double trong HTTP contract | Đạt phạm vi fixture; byte DICOM/JPEG theo khác biệt đã chốt |
| Lỗi store và thời điểm response | Store giả localhost, backend production | 26 trường hợp đạt; không phải 26 test trên STORE.md |
| Hủy/deadline trong lúc encode | Synthetic 384 frame 512×512, JavaCV/FFmpeg thật | MP4 giải mã đủ frame; file/permit giữ tới completion, TTL bỏ qua active, readiness phục hồi |
| JNI queue/deadline/reference | JVM thật, codec double cho bài điều phối | Đạt suite đã chạy; không thay kiểm chứng mọi codec |
| Workspace | Debug Windows | 20 pass, 14 ignored không tính đạt; JNI/video/HTTP liên quan chạy riêng |
| Hồi quy DB dùng chung | PostgreSQL theo TOML | 9 test auth/config/session/study/WADO đã đạt ở lượt trước |
| Build/lint | Debug Windows | Build, clippy -D warnings, format và diff check đạt ở lượt triển khai gần nhất |

Bằng chứng cục bộ:

- `target/step-12-logs/live-store/result.json`: store thật, byte/decoded JPEG và ZIP.
- `target/step-12-logs/mp4-independent.json`, `jni-real-video.log`, `http-download-contract.log`: MP4 và contract HTTP.
- `target/step-12-error-logs/results.json`: 26 bài lỗi giả lập/FFmpeg thật.
- Lệnh chạy lại trong [DEVELOPMENT_COMMANDS](../DEVELOPMENT_COMMANDS.md#8-download-dicomjpegmp4-step-12).

Store thật có WADO-URI trả body JPEG với MIME image/png; Rust dùng WADO-RS
rendered có MIME image/jpeg. Không tính WADO-URI media contract là đạt.

## Streaming và lỗi đã chốt

Rust chuẩn bị xong output trước khi gửi headers, sau đó stream file theo chunk
64 KiB với Content-Length. Thời điểm first byte/lỗi khác Java servlet streaming.

- Trước headers: nguồn non-200 hoặc body cắt trả JSON 500; deadline JNI dùng
  chung trả 504, queue JNI đầy trả 503. Không có quota download/413 hoặc deadline
  toàn request. Không gửi ZIP dở hoặc body lỗi store.
- Sau headers: status đã commit là 200; store lỗi không ảnh hưởng file đã chuẩn bị.
  Nếu file output bị cắt, client nhận biết thiếu byte qua Content-Length.
- Client ngắt khi encode: FFmpeg tiếp tục tới completion rồi mới dọn tài nguyên.
  Deadline khi native còn chạy trả 504; readiness tạm 503, health 200 và phục hồi
  khi native kết thúc. Không tạo JVM thay thế.

Các hành vi lỗi trên đã test có kiểm soát bằng nguồn giả lập; xác nhận với nguồn
thật là mục tiếp theo, không được đánh dấu đạt từ kết quả giả lập.

## Kế hoạch rút gọn cấu hình download — 2026-11-02

Theo yêu cầu người dùng: bỏ cấu hình download được thêm ở Rust so với Java.
Đã triển khai và kiểm chứng refactor ngày 2026-11-03. Các ghi nhận cũ
về bắt buộc khai báo download limits/P1–P4 không còn là yêu cầu cho phạm vi này.

### Contract hiện hành — áp dụng từ 2026-11-03

- Không có section `[download]` hoặc cờ bật download trong cấu hình ứng dụng.
- Giữ `video.fps` (TOML `[video]`, `fps = 1` mặc định) tương ứng Java
  `@Value("${video.fps:1}")`; tiếp tục dùng typed config chung của Summer.
- Giữ `DOWNLOAD_PATH_FORMAT` trong bảng `config` và fallback hiện có; không
  chuyển sang TOML, không đổi schema.
- Không thay toàn bộ tham số vừa bỏ bằng một bộ giới hạn hardcode gây ra cùng
  các hạn chế nghiệp vụ. Chỉ giữ ràng buộc kỹ thuật thực sự của thư viện/JNI,
  kiểm tra dữ liệu hợp lệ và cơ chế quản lý vòng đời tài nguyên.
- Giữ kiến trúc file-based/JNI, JPEG rendered từ store và thời điểm gửi response
  đã chốt. Không mở rộng thành viết lại download để giống Java về kiến trúc.

### Đầu việc sửa

- [x] Bỏ `DownloadSettings` khỏi registry/config và export; thay phần FPS bằng
  typed `VideoSettings` dùng `[video]`, mặc định 1. Rà soát profile đang quản lý.
- [x] Bỏ `enabled` và nhánh runtime vắng do thiếu cấu hình trong main/router;
  luôn khởi tạo runtime download. Lỗi thiếu JNI chỉ áp dụng cho thao tác cần JNI.
- [x] Bỏ các policy `max_jobs`, `timeout_ms`, `max_input_bytes`,
  `max_output_bytes`, `max_job_bytes`, `max_metadata_bytes`, `max_entries`,
  `max_frames`, `max_pixels` cùng kiểm tra/nhánh lỗi chỉ phục vụ các policy này.
  Rà cả transport, ZIP và facade Java/JNI; không dùng số cực đại giả làm
  "unlimited". Phân biệt với timeout/queue JNI dùng chung và giới hạn kiểu dữ
  liệu/thư viện, không xóa chúng theo tên một cách máy móc.
- [x] Bỏ `temp_dir`, `cleanup_ttl_seconds`, `cleanup_interval_seconds` khỏi
  cấu hình. Dùng thư mục tạm hệ thống với vùng riêng cho ứng dụng; giữ lease,
  lock và cleanup sau stream/worker completion. Quy tắc dọn file mồ côi là
  chi tiết nội bộ, không thêm tham số vận hành mới. Không quét/xóa thư mục tạm
  của ứng dụng khác.
- [x] Cập nhật fixture, test, script smoke/live/failure và profile sinh ra để
  không còn ghi section `[download]`; thay test quota tùy ý bằng kiểm tra lỗi
  thực tế còn áp dụng. Giữ kiểm tra disconnect/cancellation và file lifecycle.
- [x] Bỏ `config/download.example.toml`; cập nhật README, contract download,
  tham chiếu kỹ thuật, DEVELOPMENT_COMMANDS và nội dung Step 12 đang mô tả
  limits bắt buộc. Ghi rõ profile triển khai cũ cần bỏ `[download]` và đổi
  `download.video_fps` sang `video.fps`; lịch sử bằng chứng cũ vẫn giữ nguyên.
- [x] Xử lý hai điểm review: bỏ guard metadata 16 MiB ở cả hai chiều JNI và
  chọn ZIP64 theo kích thước entry. Không đưa số cực đại vào API thay quota.

### Tiêu chí kiểm chứng

- [x] Ứng dụng khởi động và download hoạt động khi không có `[download]` hoặc
  `[video]`; FPS mặc định 1, ghi đè `[video].fps` qua profile Summer hoạt động.
- [x] DICOM/JPEG instance chạy khi JNI tắt; thao tác cần JNI vẫn báo lỗi phù hợp.
- [x] Study/series ZIP và MP4 giữ nội dung/header/naming trong baseline hiện có;
  DOWNLOAD_PATH_FORMAT thiếu hoặc có cấu hình đều được kiểm tra.
- [x] Không còn từ chối request chỉ vì các quota download đã bỏ; kiểm tra mẫu
  vượt giới hạn synthetic cũ mà không tạo tải không kiểm soát.
- [x] File tạm/lock được giải phóng đúng sau thành công, lỗi nguồn, ngắt client
  và worker hoàn tất; file active không bị cleanup xóa.
- [x] Chạy format, build, lint, workspace tests và suite download phù hợp theo
  DEVELOPMENT_COMMANDS. Test chưa chạy/ignored được báo riêng, không tính đạt.

### Tiến độ refactor — 2026-11-03, Codex

Phần rút gọn cấu hình: `in_review`. Step 12 tổng thể vẫn `in_progress` vì corpus
codec rộng, đĩa đầy, memory/soak, release và Linux chưa có bằng chứng mới.

- Bỏ cấu hình/quota download ở Rust và facade Java; chỉ typed `[video].fps`
  mặc định 1. DICOM/JPEG chạy khi JNI tắt. JNI dùng queue/deadline chung cho từng
  lời gọi, không đặt lại một bộ quota hardcode hoặc deadline toàn request.
- File tạm dùng vùng `pacs-viewer-core-downloads` dưới TEMP hệ thống. Lease/lock
  giữ file đang stream/native encode; orphan cleanup chạy nội bộ mỗi 60 giây,
  chỉ dọn job đã bỏ khóa và cũ hơn 5 phút. Đây là vòng đời file, không quota job.
- Facade API tăng 1 → 2 vì bỏ tham số quota trong chữ ký JNI; manifest và loader
  từ chối distribution cũ. Phải build/deploy facade cùng binary Rust. Docker
  config bỏ `[download]`, dùng FPS mặc định 1; TMPDIR=/var/lib/viewer giữ file
  tạm trên named volume. Chưa chạy Docker build/runtime trên máy này.
- `cargo test --workspace --locked`: 24 pass, 14 ignored tổng hai crate. Các
  bài JNI/download có môi trường riêng được chạy qua các runner bên dưới;
  không tính các ignored khác là đạt.
- `cargo build --workspace --locked`, `cargo clippy --workspace --all-targets
  --locked -- -D warnings` và format đã đạt.
- `tools/facade.py build`: package/smoke API 2 đạt với JDK 25, sau khi cho Maven
  quyền ghi cache dependency. `tools/smoke_download.py --app --independent` đạt:
  metadata >16 MiB qua hai chiều JNI, metadata fallback/custom path, HTTP golden
  contract, MP4 Java/Rust/JNI khớp pixel sau giải mã FFmpeg độc lập. Heap riêng
  fixture lớn là 384 MiB; lượt đầu heap 128 MiB bị OOM, không tính đạt.
- `tools/smoke_jni.py`: 24 subprocess case đạt, gồm queue/deadline/cancellation,
  exception/reference và từ chối API/distribution sai. `tools/smoke_app.py` đạt
  dev/prod JNI tắt, profile/interpolation/logging, startup failure và shutdown.
- Hồi quy PostgreSQL theo filter
  `postgres_migrations_transaction_readiness_and_close -- --ignored`: 9 bài đạt
  (auth/config/client/admin/session/study/WADO và hạ tầng). Format/diff check đạt.
- `tools/test_download_failures.py`: 26 case đạt với store synthetic và FFmpeg
  thật. Store chậm headers/body hoàn tất 200; DICOM vẫn 200 khi JNI encode/quá
  deadline; native giữ lease đến completion, readiness hồi phục, disconnect và
  upstream failure dọn file. Evidence: `target/step-12-error-logs/results.json`.
- `tools/verify_live_download.py --samples 3 --java-home ...`: store chỉ có 2
  instance; cả hai DICOM/JPEG khớp byte khi JNI tắt. ZIP DICOM/JPEG study/series
  (4 tổ hợp) đạt với metadata JNI thật. Evidence:
  `target/step-12-logs/live-store/result.json`. Không kiểm chứng MP4 store thật.
- ZIP64 unit test kiểm tra local header và đọc lại ZIP ở hai phía ngưỡng 4 GiB
  mà không cấp phát file khổng lồ; chưa chạy đóng ZIP với entry thực >4 GiB.

Không commit hoặc deploy trong lượt sửa này. Khi nâng cấp: bỏ `[download]` cũ,
chuyển FPS tùy chỉnh sang `[video].fps`, triển khai binary và facade API 2 cùng nhau.

### Cập nhật tài liệu sau review — 2026-11-04, Codex

- Đồng bộ checklist với lease/vòng đời file; bỏ yêu cầu resource limits production
  khỏi tiêu chí hiện hành. Giữ các mục kiểm chứng đĩa đầy, memory/soak và corpus rộng.
- Phân biệt bảng bằng chứng trước refactor với kết quả contract hiện hành;
  quota/P1–P4 và permit download trong lịch sử không còn là yêu cầu triển khai.
- Lượt review 2026-11-03 đã chạy lại `cargo test --workspace --locked`:
  24 pass, 14 ignored; `tools/smoke_download.py --app --independent` với JDK 25
  đạt metadata/MP4/JNI, giải mã FFmpeg độc lập và HTTP download contract.
- Lượt này chỉ sửa tài liệu, không chạy thêm test runtime. Phần rút gọn cấu hình
  giữ `in_review`; Step 12 tổng thể giữ `in_progress`, không đánh dấu thêm mục đạt.
- Người dùng xác nhận commit code refactor và tài liệu ngày 2026-11-04;
  các mục chưa kiểm chứng và trạng thái Step 12 được giữ nguyên.
