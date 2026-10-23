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
| `cargo test --workspace --locked -- --include-ignored` | Chạy cả kiểm thử PostgreSQL thật: migration, transaction, readiness và đóng pool. |
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

## 5. Xem thay đổi trước khi commit

| Lệnh | Mục đích |
|---|---|
| `git status --short` | Xem file đã thêm hoặc sửa. |
| `git diff --check` | Tìm lỗi khoảng trắng trong các file đã theo dõi. |
| `git diff` | Đọc lại các thay đổi đã theo dõi trước khi commit. |

Các lệnh trên kiểm tra nền tảng HTTP và persistence. Khi có module nghiệp vụ, bổ sung test
và kiểm tra riêng vào step tương ứng.
