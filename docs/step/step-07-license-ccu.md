# Step 7 - UC-014 Theo dõi license và người dùng đồng thời

## Tính năng

Chuyển license gate và theo dõi người dùng đồng thời sang Rust; không mang logic seal sang.

**JNI:** Không.

## Nguồn Java

- [module/license/rest/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/license/rest/)
- [module/license/service/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/license/service/)
- [UC-014](../../../1pacs-viewer-core/docs/use_cases/UC-014-theo-doi-license-va-nguoi-dung-dong-thoi.md)

## Việc cần làm

- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: runtime state/config cho license và CCU; không tạo entity nghiệp vụ mới.
- [ ] Khởi tạo license state.
- [ ] Triển khai riêng tracker event-channel và heartbeat.
- [ ] Fingerprint deduplication và CCU limit.
- [ ] Heartbeat expiry 45 giây, cleanup 15 giây; event keep-alive/expiry theo cấu hình mặc định.
- [ ] Gate cung cấp Viewer theo license state.

## Kiểm tra hoàn thành

- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-014 main flow, A1..A4, BR-001..BR-007; race khi nhiều kết nối cùng fingerprint và khi đạt giới hạn.
- [ ] Đạt khi: hai tracker độc lập, snapshot tổng hợp và license gate khớp baseline.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
