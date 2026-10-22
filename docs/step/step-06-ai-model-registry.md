# Step 6 - UC-012 Quản lý model AI

## Tính năng

Chuyển CRUD model AI, cấu hình kết nối và validation sang Rust.

**JNI:** Không.

## Nguồn Java

- [module/ai/rest/ModelController.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/ai/rest/ModelController.java)
- [module/ai/service/ModelServiceImpl.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/ai/service/ModelServiceImpl.java)
- [UC-012](../../../1pacs-viewer-core/docs/use_cases/UC-012-quan-ly-model-ai.md)

## Việc cần làm

- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: MODELS.
- [ ] List/get/create/update/delete model.
- [ ] Enforce `model_name` bắt buộc/duy nhất và `url` bắt buộc.
- [ ] Bảo toàn authentication/config JSON semantics.
- [ ] Chưa tự thêm authorization vì precondition hiện ghi không có cơ chế riêng.

## Kiểm tra hoàn thành

- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-012 main flow, A1, A2, BR-001, BR-002 và unique constraint race.
- [ ] Đạt khi: model registry tương thích contract và database effect.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
