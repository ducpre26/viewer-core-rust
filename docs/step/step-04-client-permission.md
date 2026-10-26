# Step 4 - UC-009 Quản trị client và quyền

## Tính năng

Chuyển CRUD client, permission và gán quyền từ Java sang Rust.

**JNI:** Không.

## Nguồn Java

- [module/util/rest/AdminAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/util/rest/AdminAPI.java)
- [module/util/service/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/util/service/)
- [UC-009](../../../1pacs-viewer-core/docs/use_cases/UC-009-quan-tri-client-va-quyen.md)

## Việc cần làm

- [x] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [x] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: CLIENT, PERMISSION, CLIENT_PERMISSION; tái sử dụng phần có ở Step 3a.
- [x] CRUD client và permission.
- [x] Gán/bỏ gán client–permission.
- [x] Admin-only authorization.
- [x] Enforce permission name unique và composite key `(client_id, permission_id)`.
- [x] Xóa mapping liên quan khi xóa client/permission theo baseline.

## Kiểm tra hoàn thành

- [x] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [x] UC-009 main flow, A1, A2 và BR-001..BR-004; transaction rollback và orphan mapping.
- [x] Đạt khi: admin CRUD/mapping contract tests và database postcondition pass.

## Tiến độ

- Trạng thái: `done`
- Đã làm: Port 13 thao tác HTTP client/permission/mapping, admin-only guard, BCrypt cost 8/version 2a, cache credential invalidation; migration permission/client_permission cùng unique name/composite PK/FK cascade, seed ADMIN/MODIFY_OVERLAY_CONFIG. Tái sử dụng client/session Step 3a.
- Baseline: Java revision `c983e4eac389fa6b31f970cfee8f9fce5e20af4c`, [contract và fixture](step-04-baseline.md). Không có job thuộc tính năng này.
- Kiểm tra thực tế Windows local: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` (4 unit pass), `cargo build --workspace --locked`; lệnh PostgreSQL hiện có trong DEVELOPMENT_COMMANDS chạy đủ 3 bộ test admin/auth-session/migration-readiness, 0 fail, 0 skipped; `python tools/smoke_app.py` đạt dev/prod, startup failures và shutdown.
- Contract tests: UC-009 main flow/A1/A2/BR-001..004; CRUD, validation, missing resource, merge cùng ID, password trim/hash/không trả password, update null/blank, pagination, admin-only mọi thao tác, client có ADMIN vẫn bị từ chối, disabled không bỏ admin, unique name, mapping thiếu/trùng/đồng thời, FK reject, rollback cập nhật/transaction, cascade hai phía và credential cache invalidation. Hồi quy auth/session Step 3a đạt.
- Evidence: `target/step-04-logs/postgres-regression.log`, `app-regression.log`, `test-db-final-state.log`, `test-db-stop.log`. PostgreSQL 18 tạm đã dừng/xóa; DB cuối chỉ có client/session/permission/client_permission/seaql_migrations, 0 kết nối ứng dụng. Harness tạm giữ nhãn PASS JNI cũ, không tính nhãn đó là bằng chứng JNI; không commit harness/log/DB.
- Giới hạn: Baseline đối chiếu source Java, không chạy Spring HTTP; chưa chạy CI Ubuntu/release, không chạy lại JNI vì Step 4 không sửa JNI. Lỗi driver dùng message an toàn; lỗi HTTP do framework tạo không coi là business contract. Khác biệt UC A1/catch lỗi mapping ghi tại baseline; giữ code Java theo quy tắc migration.
- Tiếp theo: Người dùng đã xác nhận commit `feat(step-04): implement client and permission administration`; không push hoặc tự chuyển Step 5.
- Vướng mắc: Không có.
- Cập nhật: 2026-10-26 — Codex; người dùng xác nhận tạo commit Step 4.
