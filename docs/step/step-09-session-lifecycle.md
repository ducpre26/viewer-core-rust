# Step 9 - UC-002 Quản lý và chia sẻ phiên 2D

## Tính năng

Chuyển quản lý, chia sẻ và dọn phiên 2D sang Rust.

**JNI:** Không.

## Nguồn Java

- [module/session/rest/SessionAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/session/rest/SessionAPI.java)
- [module/session/service/SessionServiceImpl.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/session/service/SessionServiceImpl.java)
- [module/session/job/SessionScheduler.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/session/job/SessionScheduler.java)
- [UC-002](../../../1pacs-viewer-core/docs/use_cases/UC-002-quan-ly-va-chia-se-phien-2d.md)

## Việc cần làm

- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: SESSION và CONFIG; hoàn thiện lưu/chia sẻ/cleanup long-term.
- [ ] Session ID extractor từ path, `session` và `sessionId`.
- [ ] Short-term lookup trước, PostgreSQL fallback sau.
- [ ] Get/update/end session.
- [ ] Tạo long-term shared session, anonymous option và share key sáu chữ số.
- [ ] Cleanup session hết hạn lúc nửa đêm Asia/Ho_Chi_Minh.
- [ ] Tái sử dụng extractor/lookup Step 3a và mở rộng lifecycle; không triển khai lại một auth path độc lập.

## Kiểm tra hoàn thành

- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-002 main flow, A1..A5, BR-001..BR-007; cache miss/db hit/expired/delete và idempotent end-session.
- [ ] Đạt khi: toàn bộ session lifecycle và scheduler behavior khớp baseline.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
