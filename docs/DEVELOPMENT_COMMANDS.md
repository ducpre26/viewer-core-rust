# Lệnh phát triển Viewer Core Rust

Chạy các lệnh dưới đây từ thư mục gốc repo `1pacs-viewer-core-rust`. Toolchain Rust
được ghim trong `rust-toolchain.toml`; dependency được khóa trong `Cargo.lock`.

## 1. Chuẩn bị môi trường

| Lệnh | Mục đích |
|---|---|
| `rustup show` | Xem toolchain và target Rust đang được chọn. |
| `cargo --version` | Kiểm tra Cargo đã sẵn sàng. |

Cần có Python 3 để chạy smoke test. Trên Windows cần MSVC C++ Build Tools; trên
Linux x86_64 cần trình biên dịch C/C++ và linker.

Từ Step 02, PostgreSQL bắt buộc. Trước khi chạy app hoặc kiểm thử tích hợp,
đặt `VIEWER_DATABASE_URL=postgresql://127.0.0.1:5432/viewer_rust`,
`VIEWER_DATABASE_USER=postgres` và `VIEWER_DATABASE_PASSWORD` trong môi trường.
Không lưu mật khẩu vào repo. Migration tự chạy trước khi mở HTTP;
kết nối DB thất bại thì ứng dụng dừng với mã lỗi.

## 2. Chạy khi phát triển

| Lệnh | Mục đích |
|---|---|
| `cargo run --locked -p viewer-app` | Build debug rồi chạy ứng dụng; dừng bằng `Ctrl+C`. |
| `cargo check --workspace --locked` | Kiểm tra biên dịch nhanh sau khi sửa code, không tạo bản chạy hoàn chỉnh. |
| `cargo fmt --all` | Tự định dạng mã Rust. |

Ứng dụng mặc định nghe ở `127.0.0.1:8888`. Nếu cổng 8888 đã được Java backend sử dụng:

```powershell
# Windows PowerShell: chọn cổng khác cho phiên shell hiện tại.
$env:VIEWER_PORT = '8889'
cargo run --locked -p viewer-app
```

```bash
# Linux: chọn cổng khác cho một lần chạy.
VIEWER_PORT=8889 cargo run --locked -p viewer-app
```

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
| `cargo test -p viewer-app --locked postgres_migrations_transaction_readiness_and_close -- --ignored` | Chạy PostgreSQL thật: migration/transaction/readiness, auth/session HTTP/cache (Step 3a), admin CRUD/mapping, unique/FK/cascade/rollback và cache credential (Step 4), đóng pool. Cần DB riêng cho test. |
| `cargo build --workspace --locked` | Build debug cho toàn workspace. |
| `python tools/smoke_app.py` | Chạy ứng dụng debug thật để kiểm tra HTTP, cấu hình, logging và dừng sạch. |

Smoke test tự dùng cổng localhost trống và chạy cả cấu hình dev/prod. Không cần
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
- Artifact: `java/dicom-facade/target/distribution/`; dependency directory hiện rỗng.
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
`--app` cần biến môi trường PostgreSQL và binary debug đã build; thêm `--release`
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
