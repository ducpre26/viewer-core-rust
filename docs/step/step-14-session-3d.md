# Step 14 - UC-007 Quản lý phiên dựng hình 3D

## Tính năng

Chuyển quản lý phiên 3D, Python/Trame process pool và WebSocket sang Rust.

**JNI:** Không.

## Nguồn Java

- [module/session3d/rest/Session3DAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/session3d/rest/Session3DAPI.java)
- [module/session3d/service/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/session3d/service/)
- [UC-007](../../../1pacs-viewer-core/docs/use_cases/UC-007-quan-ly-phien-dung-hinh-3d.md)

## Việc cần làm

- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: SESSION và runtime registry/process; không thêm bảng session 3D.
- [ ] Pre-warmed Python/Trame process pool và port allocation.
- [ ] Create/list/get/delete/reset session 3D và available ports.
- [ ] WebSocket server tới Viewer và WebSocket client tới Trame.
- [ ] Validate cặp session 2D/3D.
- [ ] Watchdog attach timeout, reclaim process mỗi phút và idempotent cleanup.

## Kiểm tra hoàn thành

- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-007 main flow, A1..A9, BR-001..BR-004; process crash, port exhaustion, reconnect, disconnect-before-attach và reset semantics.
- [ ] Đạt khi: resource không leak, registry/API/WebSocket behavior khớp baseline.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
