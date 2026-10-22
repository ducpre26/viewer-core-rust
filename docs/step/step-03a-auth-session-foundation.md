# Step 3a - Auth/session foundation

## Tính năng

Chuyển auth, session lookup và kiểm tra quyền sang middleware Rust trước các API nghiệp vụ.

**JNI:** Không.

## Nguồn Java

- [config/WebRequestFilter.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/config/WebRequestFilter.java)
- [config/WebSecurityConfig.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/config/WebSecurityConfig.java)
- [module/util/permission/SessionPermissionEvaluator.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/util/permission/SessionPermissionEvaluator.java)

## Việc cần làm

- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: CLIENT, SESSION và dữ liệu quyền cần cho auth/lookup; chỉ mapping phần cần thiết, dùng client/session fixture để test, không cần API tạo session đã hoàn tất.
- [ ] Port credential validation, Basic Auth, IP allow-list và security configuration theo route inventory/baseline, bao gồm admin routes.
- [ ] Tạo session ID extractor từ path/query với precedence và ngoại lệ theo baseline; short-term lookup trước, PostgreSQL riêng của Rust fallback sau.
- [ ] Thiết lập security context, permission guard/extractor và ADMIN bypass; bảo toàn permission snapshot semantics.
- [ ] Dùng client/session fixture để kiểm thử middleware trước khi API tạo và quản lý session hoàn tất; không coi foundation là hoàn thành UC-001/UC-002.

## Kiểm tra hoàn thành

- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] HTTP qua middleware thật: valid/invalid credentials, IP policy, missing/expired session, cache miss/DB hit, admin/non-admin, permission allow/deny; không bypass auth.
- [ ] Đạt khi: test qua HTTP cho valid/invalid credentials, IP policy, missing/expired session, cache miss/DB hit, admin/non-admin và permission allow/deny pass bằng middleware thật, không bypass auth trong test CRUD.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
