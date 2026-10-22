# Step 5 - UC-008 Quản lý cấu hình Viewer

## Tính năng

Chuyển CRUD cấu hình Viewer và quyền sửa overlay sang Rust.

**JNI:** Không.

## Nguồn Java

- [module/util/rest/ConfigPublicAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/util/rest/ConfigPublicAPI.java)
- [module/util/rest/AdminAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/util/rest/AdminAPI.java)
- [module/util/service/ConfigServiceImpl.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/util/service/ConfigServiceImpl.java)
- [UC-008](../../../1pacs-viewer-core/docs/use_cases/UC-008-quan-ly-cau-hinh-viewer.md)

## Việc cần làm

- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: CONFIG.
- [ ] List/get/create/update/delete config.
- [ ] Validate `id` và `value` bắt buộc.
- [ ] Implement `MODIFY_OVERLAY_CONFIG` và ADMIN bypass.
- [ ] Cache config và invalidation sau write.

## Kiểm tra hoàn thành

- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-008 main flow, A1, A2, BR-001 và BR-002; cache invalidation.
- [ ] Đạt khi: config API/permission/database/cache postcondition pass.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
