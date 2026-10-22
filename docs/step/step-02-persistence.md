# Step 2 - Kết nối PostgreSQL và nền tảng persistence

## Tính năng

Sau Step 1, kết nối PostgreSQL riêng bằng SeaORM. Chưa mapping tám entity hoặc import toàn bộ schema Java.

**JNI:** Không.

## Nguồn Java

Chưa cần đối chiếu schema Java. Đọc [entity model](../../../1pacs-viewer-core/docs/entity_model.md) và schema theo từng tính năng từ Step 3a trở đi.

## Việc cần làm

- [ ] Cấu hình database URL/credential qua môi trường, kết nối và connection pool SeaORM; không log secret.
- [ ] Chuẩn bị migration runner và quy ước repository/transaction; chưa tạo bảng nghiệp vụ.
- [ ] Đưa kiểm tra kết nối DB vào readiness khi DB được bật; đóng pool khi shutdown.
- [ ] Để step đầu tiên cần entity chịu trách nhiệm mapping schema/migration/repository của entity đó; step sau tái sử dụng hoặc mở rộng có kiểm thử.

## Kiểm tra hoàn thành

- [ ] Kết nối/query SELECT 1 thành công trên PostgreSQL riêng; sai credential hoặc DB không truy cập được phải báo lỗi rõ.
- [ ] Migration runner chạy được với chưa có migration nghiệp vụ; readiness/shutdown đúng.
- [ ] Không yêu cầu CRUD/constraint test cho tám entity ở step này; test đó đi cùng tính năng tạo/sử dụng entity.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Thực hiện checklist trong phạm vi step này; không chờ baseline toàn bộ Java.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — đổi sang init trước, đối chiếu theo tính năng; chưa thực hiện implementation.
