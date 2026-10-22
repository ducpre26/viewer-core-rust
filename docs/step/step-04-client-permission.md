# Step 4 - UC-009 Quản trị client và quyền

## Tính năng

Chuyển CRUD client, permission và gán quyền từ Java sang Rust.

**JNI:** Không.

## Nguồn Java

- [module/util/rest/AdminAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/util/rest/AdminAPI.java)
- [module/util/service/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/util/service/)
- [UC-009](../../../1pacs-viewer-core/docs/use_cases/UC-009-quan-tri-client-va-quyen.md)

## Việc cần làm

- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: CLIENT, PERMISSION, CLIENT_PERMISSION; tái sử dụng phần có ở Step 3a.
- [ ] CRUD client và permission.
- [ ] Gán/bỏ gán client–permission.
- [ ] Admin-only authorization.
- [ ] Enforce permission name unique và composite key `(client_id, permission_id)`.
- [ ] Xóa mapping liên quan khi xóa client/permission theo baseline.

## Kiểm tra hoàn thành

- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-009 main flow, A1, A2 và BR-001..BR-004; transaction rollback và orphan mapping.
- [ ] Đạt khi: admin CRUD/mapping contract tests và database postcondition pass.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
