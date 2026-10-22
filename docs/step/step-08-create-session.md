# Step 8 - UC-001 Khởi tạo phiên xem ảnh 2D

## Tính năng

Chuyển API tạo session 2D và URL Viewer sang Rust.

**JNI:** Không.

## Nguồn Java

- [client/ClientAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/client/ClientAPI.java)
- [module/session/service/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/session/service/)
- [UC-001](../../../1pacs-viewer-core/docs/use_cases/UC-001-khoi-tao-phien-xem-anh-2d.md)

## Việc cần làm

- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: CLIENT, permission, CONFIG và SESSION; tái sử dụng repository đã có, hoàn thiện phần tạo session.
- [ ] Basic Auth cho `/client/**` và IP allow-list.
- [ ] Validate store/studies.
- [ ] Snapshot permission tại thời điểm tạo session.
- [ ] Normalize expiry theo config.
- [ ] Tạo short-term session và viewer URL.
- [ ] Tái sử dụng auth/session foundation Step 3a; không tạo cơ chế credential/permission thứ hai.

## Kiểm tra hoàn thành

- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-001 main flow, A1, A2, BR-001..BR-003; credential/IP/expiry/permission snapshot.
- [ ] Đạt khi: session/link/permission/database-cache postcondition pass.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
