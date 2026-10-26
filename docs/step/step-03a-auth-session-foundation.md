# Step 3a - Auth/session foundation

## Phạm vi

Chuyển auth, session lookup và kiểm tra quyền sang middleware Rust. Không dùng JNI; chưa triển khai CRUD client/quyền hoặc API tạo, chia sẻ, cập nhật, xóa session.

Đối chiếu mọi luồng endpoint từ mã Java: controller → filter/security → service → repository/cache; ghi nguồn, kết quả mong đợi, lỗi và tác động dữ liệu. Không tạo/chạy test baseline Java. Chỉ kiểm thử implementation Rust; phần chưa xác định từ mã phải ghi rõ, không tự đổi nghiệp vụ.

## Nguồn Java

Revision: `c983e4eac389fa6b31f970cfee8f9fce5e20af4c`.

- [WebRequestFilter](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/config/WebRequestFilter.java), `WebSecurityConfig`, `WebAuthenticationEntryPoint`, `SecurityUtil`, `PasswordEncoder`.
- [SessionServiceImpl](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/session/service/SessionServiceImpl.java), `ShortTermSessionService`, `LongTermSessionServiceImpl`, `SessionGetDto`.
- [SessionPermissionEvaluator](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/util/permission/SessionPermissionEvaluator.java), `PermissionCheckService`; controller `ClientAPI`, `AdminAPI`, `SessionAPI`, `ConfigPublicAPI`.
- Entity/repository `Client`, `Session`, [schema](../../../1pacs-viewer-core/db/init.sql), `EhCacheConfig` (Caffeine thực tế, không dùng cấu hình Ehcache cũ).

## Việc cần làm

- [x] Đối chiếu từng endpoint và mọi nhánh liên quan từ mã Java; ghi vị trí nguồn và kết quả mong đợi trong tài liệu này.
- [x] Tạo migration, entity/repository SeaORM cho client, session và dữ liệu quyền cần cho auth; giữ schema Java, không chạy script xóa bảng.
- [x] Port Basic Auth, BCrypt, IP allow-list, cấu hình security và admin-role guard.
- [x] Port session ID extractor, share-key validation, cache-first/DB-fallback, expiry và invalidation nội bộ.
- [x] Tạo request security context và permission guard; giữ permission snapshot, phân biệt role admin và permission ADMIN.
- [x] Kiểm thử Rust bằng client/session tổng hợp qua middleware thật; không thêm route seed hoặc bypass auth vào production.

## Điểm tương thích cần giữ

- ID lấy từ path; evaluator đọc biến `sessionId`, không tự thêm query fallback. Giữ phạm vi filter và ngoại lệ URI kết thúc bằng ID.
- Admin Basic vẫn bắt buộc khi tắt client security; session validation vẫn chạy. IP ưu tiên nguyên X-Forwarded-For và so bằng chuỗi contains.
- Cache expired: evict và trả null, không fallback DB trong cùng lượt gọi. DB hit hợp lệ: warm cache; DB expired: xóa. Chỉ hợp lệ khi expiredTime > now.
- Giữ share-key, snapshot và các nhánh bypass theo mã. Cache session expire-after-write 120 phút, tối đa 1000 phần tử; TTL cache khác hạn session.

## Kiểm tra hoàn thành

- [x] HTTP Rust bao phủ credential đúng/sai/sai định dạng, IP allow/deny, admin/non-admin, security bật/tắt, nguồn ID, share key và permission allow/deny/ADMIN; không bypass middleware.
- [x] Cache/DB bao phủ hit/miss, warm/evict, session thiếu/hết hạn/đúng thời điểm hết hạn, không fallback sau cache expired và permission snapshot.
- [x] PostgreSQL thật xác nhận migration, constraint, tác động dữ liệu và rollback khi liên quan.
- [x] Kiểm tra CORS, lỗi, context giữa request; hồi quy health/readiness/OpenAPI và startup/shutdown; hồi quy JNI khi đổi phần tích hợp.
- [x] Chạy format, clippy, test/build và smoke phù hợp theo [DEVELOPMENT_COMMANDS.md](../DEVELOPMENT_COMMANDS.md); ghi kết quả thực tế, không tính test bỏ qua là đạt.

## Tiến độ

- Trạng thái: `done` — implementation và kiểm tra Windows local đạt; CI Windows/Ubuntu chưa chạy, nếu phát hiện lỗi sẽ sửa lại step.
- Đã làm: Migration/entity client/session, middleware/guard, cache và test Rust; khóa transaction PostgreSQL tránh migration đồng thời. Giữ schema, snapshot, nguồn ID và các ngoại lệ Java. Rust gốc: `80b6edac5e4859702683e1436b946c70ada6c4bb`.
- Tiếp theo: Theo dõi CI khi workflow chạy; chỉ chuyển Step 4 khi người dùng yêu cầu.
- Giới hạn: Chỉ đối chiếu mã Java, không chạy baseline Java. Body lỗi do servlet container sinh không được xem là business contract; Rust dùng text an toàn cùng HTTP status. Chưa có kết quả CI Ubuntu.
- Kiểm tra thực tế (2026-10-26): format/clippy/workspace test đạt (3 nhóm unit); lệnh PostgreSQL đã ghi trong DEVELOPMENT_COMMANDS chạy đủ 2 test, gồm auth HTTP qua middleware thật/TCP, schema/default/index/constraint, rollback, expiry, snapshot, cache invalidation, CORS và context đồng thời. Debug/release build và smoke dev/prod đạt; hồi quy JNI debug/release qua app đạt. Test ignored cần DB/JNI đã chạy riêng đúng môi trường, không tính lượt skip là bằng chứng.
- Evidence: `target/step-03a-logs/` có `build-debug.log`, `postgres-regression.log`, `app-regression.log`, `app-release.log`, `step03-app-suite.log`, `test-db-final-state.log`, `test-db-stop.log`; chi tiết JNI tại `target/step-03-logs/`. PostgreSQL 18 tạm đã dừng/xóa; DB cuối chỉ có client/session/seaql_migrations, 0 kết nối ứng dụng. Log và DB tạm không commit.
- Cập nhật: 2026-10-26 — Codex; người dùng đã xác nhận tạo commit Step 3a, không push.

## Đối chiếu endpoint và nhánh dùng chung

Các route nghiệp vụ dưới đây chỉ là nguồn đối chiếu; Step 3a cung cấp middleware/guard, chưa triển khai handler nghiệp vụ.

| Endpoint Java | Nguồn quyết định | Kết quả cần giữ ở Rust |
|---|---|---|
| POST `/ws/rest/client/link`, `/getlink` | `ClientAPI`, `WebRequestFilter.isValidClient/doFilterInternal` | Basic khi security bật; sai header/Base64/client/password/IP → 401; sysadmin → role admin; DB client → role client. Disabled bỏ Basic client. Tạo session/quyền lúc tạo để Step 8. |
| Mọi method `/ws/rest/v1/admin/**` | `AdminAPI` class guard, `WebRequestFilter` | Luôn Basic, kể cả disabled; credential sai → 401, client thường → 403; chỉ role admin qua guard. Permission ADMIN trong session không cấp role admin. |
| GET/DELETE `/ws/rest/v1/session/{sessionID}` | `SessionAPI`, `WebRequestFilter.isValidSession` | URI kết thúc ID được filter chuyển tiếp không lookup/share-key; controller GET trả 400 nếu thiếu phiên, xử lý CRUD thuộc Step 9. |
| POST `/ws/rest/v1/session/{sessionID}/share` và các route con session | `WebRequestFilter.isValidSession`, `SessionServiceImpl.getSession` | Lookup trước handler: thiếu/hết hạn → 401; share-key có giá trị phải khớp header; lookup lỗi DB → lỗi server. Giữ nhánh `endsWith(id)` kể cả URI có suffix trùng ID. |
| GET/GET-by-id/DELETE config; POST/PUT `/ws/rest/v1/session/{sessionId}/config` | `ConfigPublicAPI`, `SessionPermissionEvaluator.getSessionIdFromRequest`, `PermissionCheckService.hasPermission` | Tất cả qua session filter; chỉ POST/PUT có guard MODIFY_OVERLAY_CONFIG. Guard lấy đúng biến sessionId; thiếu/blank/tên sessionID/query-only không thay thế. Snapshot có quyền yêu cầu hoặc ADMIN → cho phép, thiếu/null/rỗng → từ chối. Disabled bypass guard nhưng không bỏ session filter. CRUD để Step 5. |
| Route ngoài `/ws/rest/v1/`, `/ws/rest/client/` | `WebRequestFilter.shouldNotFilter`, `WebSecurityConfig` | Không tự yêu cầu đăng nhập; giữ health/readiness/OpenAPI. CORS chạy trước auth; method/header theo cấu hình Java. |

Nhánh credential: `SecurityUtil.decodeBasicAuth` tách dấu `:` đầu tiên, chấp nhận Base64 thiếu padding; `PasswordEncoder` BCrypt cost 8; hash sai/malformed bị từ chối. IP null/rỗng không giới hạn, còn lại dùng nguyên XFF hoặc remote IP và `contains`, không tự tách proxy/CIDR. Nhánh password null chỉ tồn tại trong mã, schema NOT NULL vẫn được giữ.

Nhánh lookup: `SessionServiceImpl.getSession` giữ thứ tự cache/expiry/DB/warm/delete như mục tương thích. `EhCacheConfig` còn cache client TTL 600 phút/capacity 10; invalidate khi Step 4 đổi credential/IP. Snapshot session không query join table mỗi request. `WebSecurityConfig` tắt CSRF/HTTP Basic mặc định, stateless; guard bị từ chối trả 401 khi anonymous, 403 khi đã xác thực. Request context và tracing span thay MDC, không lưu principal qua request; không ghi credential/share-key/store-auth vào log.
