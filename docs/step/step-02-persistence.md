# Step 2 - Kết nối PostgreSQL và nền tảng persistence

## Tính năng

Sau Step 1, kết nối PostgreSQL riêng bằng SeaORM. Chưa mapping tám entity hoặc import toàn bộ schema Java.

**JNI:** Không.

## Nguồn Java

Chưa cần đối chiếu schema Java. Đọc [entity model](../../../1pacs-viewer-core/docs/entity_model.md) và schema theo từng tính năng từ Step 3a trở đi.

## Việc cần làm

- [x] Cấu hình database URL/credential qua môi trường, kết nối và connection pool SeaORM; không log secret.
- [x] Chuẩn bị migration runner và quy ước repository/transaction; chưa tạo bảng nghiệp vụ.
- [x] Đưa kiểm tra kết nối DB vào readiness khi DB được bật; đóng pool khi shutdown.
- [x] Để step đầu tiên cần entity chịu trách nhiệm mapping schema/migration/repository của entity đó; step sau tái sử dụng hoặc mở rộng có kiểm thử.

## Kiểm tra hoàn thành

- [x] Kết nối/query SELECT 1 thành công trên PostgreSQL riêng; sai credential hoặc DB không truy cập được phải báo lỗi rõ.
- [x] Migration runner chạy được với chưa có migration nghiệp vụ; readiness/shutdown đúng.
- [x] Không yêu cầu CRUD/constraint test cho tám entity ở step này; test đó đi cùng tính năng tạo/sử dụng entity.

## Tiến độ

- Trạng thái: `done` — hoàn tất implementation và kiểm thử Windows local.
- Quyết định người dùng 2026-10-23: DB bắt buộc; không kết nối được thì dừng ứng dụng; migration tự chạy trước HTTP. Đã tạo database riêng `viewer_rust` trên local port 5432; credential chỉ cấp qua môi trường.
- Đã làm: SeaORM/sea-orm-migration 1.1.20 khóa trong Cargo.lock; pool dùng chung, cấu hình URL/credential/pool/timeout qua môi trường, lỗi không lộ secret. Startup chạy SELECT 1 và migration; readiness query DB có timeout trả 200/503; pool đóng sau HTTP/scheduler shutdown. Quy ước repository/transaction tại README.
- Schema: chỉ bảng tracking `seaql_migrations`, 0 migration nghiệp vụ. Không import schema Java hoặc mapping tám entity; mỗi step tính năng chịu trách nhiệm entity/migration/repository và kiểm thử liên quan.
- Kiểm tra đã đạt trên Windows: format, clippy; PostgreSQL integration test (migration chạy lại, transaction rollback, readiness 200/503 và đóng pool); smoke debug dev/prod (HTTP/OpenAPI/logging/scheduler/Ctrl+C/pool close; thiếu URL/sai mật khẩu/DB không truy cập được). Truy vấn độc lập xác nhận SELECT 1, chỉ tracking table, 0 migration đã áp dụng và 0 kết nối app còn lại sau shutdown.
- CI: bổ sung PostgreSQL 18 riêng và integration test cho Windows/Ubuntu; chưa có kết quả CI của thay đổi này.
- Kiểm tra release: `cargo build --workspace --release --locked` đạt; `python tools/smoke_app.py --release` đạt dev/prod và toàn bộ trường hợp lỗi startup, không lộ credential. Format/clippy và integration test được chạy lại sau thay đổi cuối cùng, đều đạt; `git diff --check` đạt.
- Tiếp theo: Đối chiếu CI Windows/Ubuntu khi workflow chạy; Step 3 chỉ triển khai khi người dùng yêu cầu.
- Vướng mắc: Không có vướng mắc implementation; Linux cần xác minh bằng CI.
- Cập nhật: 2026-10-23.
