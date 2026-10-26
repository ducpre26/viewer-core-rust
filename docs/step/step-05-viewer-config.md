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

- [x] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [x] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: CONFIG.
- [x] List/get/create/update/delete config.
- [x] Validate `id` và `value` bắt buộc.
- [x] Implement `MODIFY_OVERLAY_CONFIG` và ADMIN bypass.
- [x] Cache config và invalidation sau write.

## Kiểm tra hoàn thành

- [x] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [x] UC-008 main flow, A1, A2, BR-001 và BR-002; cache invalidation.
- [x] Đạt khi: config API/permission/database/cache postcondition pass.

## Tiến độ

- Trạng thái: `done`
- Đã làm: Port 10 thao tác HTTP config ở admin/session; migration CONFIG đúng schema Java; validate create, partial update, merge khi POST cùng ID; quyền MODIFY_OVERLAY_CONFIG/ADMIN và bypass disabled theo Step 3a. Cache chung TTL 600 phút, create put/update-delete evict, không cache null; đồng bộ lookup/write tránh cache fill cũ sau invalidation. Tách envelope/error/pagination/clean text dùng chung với Step 4, không đổi contract admin cũ.
- Baseline: Java revision `c983e4eac389fa6b31f970cfee8f9fce5e20af4c`, [contract và fixture](step-05-baseline.md). Không có job/JNI/file side effect trong phạm vi này. Hai file test Config Java hiện rỗng; baseline đối chiếu source, chưa chạy Spring HTTP.
- Kiểm tra thực tế Windows local: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` (4 unit pass), `cargo build --workspace --locked`; lệnh PostgreSQL hiện có trong DEVELOPMENT_COMMANDS chạy đủ 4 bộ config/admin/auth-session/migration-readiness, 4 pass, 0 fail, 0 skipped; `python tools/smoke_app.py` đạt dev/prod, startup failures và shutdown.
- Contract tests: UC-008 main flow/A1/A2/BR-001..002; CRUD cả hai prefix, create validation/trim/default datatype/enum, merge cùng ID, update null/blank/raw datatype, missing resource, pagination; bảo vệ mọi thao tác admin, client có ADMIN vẫn bị từ chối, session permission snapshot/ADMIN bypass/query spoof/disabled/expired/shared-key. Giữ DELETE không yêu cầu quyền sửa overlay theo Java. Test DB PK/NOT NULL/độ dài/rollback; lỗi write không đổi dữ liệu hay đưa giá trị lỗi vào cache. Test cache hit, list bỏ qua cache, không cache null, invalidation chéo admin/session, partial update đồng thời và đọc/ghi đồng thời. Hồi quy Step 3a/4 đạt.
- Evidence: `target/step-05-logs/postgres-regression.log`, `app-regression.log`, `test-db-final-state.log`, `test-db-stop.log`. PostgreSQL 18 tạm đã dừng/xóa; DB cuối có config cùng các bảng nền tảng, 0 kết nối ứng dụng. Harness/log/DB tạm không commit.
- Giới hạn: Chưa chạy CI Ubuntu/release; không chạy lại JNI vì Step 5 không sửa JNI. Cache Moka 10 entry theo heap Java, không tái tạo offheap 50MB; baseline ghi rõ khác biệt dung lượng. Message lỗi driver/JSON do framework tạo không coi là business contract.
- Tiếp theo: Người dùng đã xác nhận commit `feat(step-05): implement viewer configuration management`. Không push hoặc tự chuyển Step 6.
- Vướng mắc: Không có.
- Cập nhật: 2026-10-26 — Codex; hoàn tất triển khai và kiểm chứng Step 5, người dùng xác nhận tạo commit.
