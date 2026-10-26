# Step 1 - Khởi tạo Rust application

> Refactor TOML ngày 2026-10-26: module nhận typed config từ registry Summer;
> giữ app.toml/profile/interpolation/RUST_LOG theo xác nhận người dùng.
> Checklist/evidence bên dưới là lịch sử step; kết quả refactor mới nằm trong
> [mục cấu hình](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#toml-config).

## Tính năng

Khởi tạo Rust theo công nghệ đã chốt. Có thể bắt đầu ngay; không cần chạy Java, đối chiếu API nghiệp vụ hay hoàn tất Step 0.

**JNI:** Không.

## Nguồn Java

Không cần khảo sát Java ở bước init. Dùng [quyết định T1–T6](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#decisions).

## Việc cần làm

- [x] Tạo Cargo workspace và ranh giới module API/domain/persistence/auth/DICOM; module nghiệp vụ chưa có chỉ là khung.
- [x] Bootstrap Summer-rs/Axum, cấu hình môi trường, logging tracing, health/readiness và graceful shutdown.
- [x] Tích hợp OpenAPI Summer-rs cho route hạ tầng, Serde và summer-job; chưa yêu cầu khớp DTO/error nghiệp vụ Java.
- [x] Khóa phiên bản tương thích trong Cargo.lock; thiết lập format/clippy/test/build cho dev Windows và production Linux x86_64.
- [x] Readiness chỉ kiểm tra dependency đang bật/đã triển khai; chưa yêu cầu JVM hoặc codec khi JNI chưa được đưa vào.

## Kiểm tra hoàn thành

- [x] Ứng dụng build/start/stop sạch; health và OpenAPI hạ tầng hoạt động trên Windows.
- [x] Cấu hình, logging và CI smoke test pass; không cần Java runtime, fixture hay inventory để đóng step. Kết quả CI Windows/Linux và review được người dùng xác nhận ngày 2026-10-23; chưa có URL run để đối chiếu độc lập.

## Tiến độ implementation trước quyết định TOML

- Trạng thái: `done`
- Đã làm: Workspace `crates/viewer-app`, năm ranh giới module, Summer WebPlugin + JobPlugin, cấu hình dev/prod và biến môi trường, tracing JSON ở prod, route `/ws/rest/health`, `/ws/rest/ready`, `/ws/rest/docs/openapi.json`. Chưa đăng ký job nghiệp vụ hoặc adapter DB/JVM. Hướng dẫn tại [README](../../README.md).
- Phiên bản implementation: Rust 1.99.0, summer 0.7.2, summer-web 0.7.3, summer-job 0.7.1; toàn bộ dependency khóa trong [Cargo.lock](../../Cargo.lock). OpenAPI kéo theo aide 0.16.0-alpha.4; đây là dependency prerelease của summer-web, cần lưu ý khi nâng cấp. Lựa chọn này dựa trên build/smoke thực tế, không coi latest tự động được duyệt cho production.
- Kiểm tra đã đạt trên Windows x86_64: `cargo build --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` (chưa có unit test; 0 test, không dùng làm bằng chứng hành vi). `python tools/smoke_app.py` pass dev/prod: HTTP health/readiness, schema OpenAPI, 404, scheduler startup, JSON logs và Ctrl+C exit 0. Smoke test là kiểm thử hành vi thực tế.
- Kiểm tra release đã đạt trên Windows x86_64: `cargo build --workspace --release --locked` thành công; `python tools/smoke_app.py --release` pass đầy đủ ở cả dev/prod, tiến trình dừng sạch với exit 0. `python -m py_compile tools/smoke_app.py` cũng pass.
- Tiếp theo: Bắt đầu [Step 2](step-02-persistence.md) khi được yêu cầu; lưu link/ID run CI vào step này nếu cần truy vết về sau.
- Vướng mắc: Không còn vướng mắc để đóng Step 1. Máy local chưa có Linux/WSL; kết quả CI Linux do người dùng xác nhận, chưa được Codex đối chiếu với URL run.
- Cập nhật: 2026-10-23 — người dùng xác nhận review chấp thuận và CI smoke test Windows/Linux đều đạt; Codex chuyển Step 1 sang `done` theo xác nhận đó.
