# Step 9 - UC-002 Quản lý và chia sẻ phiên 2D

## Tính năng

Chuyển quản lý, chia sẻ và dọn phiên 2D sang Rust.

**JNI:** Không.

## Nguồn Java

- [module/session/rest/SessionAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/session/rest/SessionAPI.java)
- [module/session/service/SessionServiceImpl.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/session/service/SessionServiceImpl.java)
- [module/session/job/SessionScheduler.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/session/job/SessionScheduler.java)
- [UC-002](../../../1pacs-viewer-core/docs/use_cases/UC-002-quan-ly-va-chia-se-phien-2d.md)

## Hành vi Java cần giữ khi port

Đối chiếu mã nguồn ngày 2026-10-27 tại Java revision `c983e4eac389fa6b31f970cfee8f9fce5e20af4c`. Đây là contract và expected result từ source, không phải output thu được khi chạy Java. Checklist bên dưới tách đối chiếu source, kiểm thử Rust và so sánh Java runtime để mỗi dấu tick phản ánh đúng bằng chứng.

| Phạm vi | Contract và expected result cần kiểm chứng |
|---|---|
| Thời hạn chia sẻ | `SessionShareDto.expireIn` bắt buộc, đơn vị phút. Thiếu/null bị validation từ chối; giá trị <= 0 hoặc > `viewer.maximum-share-minus` (mặc định 129600 phút) được thay bằng 3600 giây. Giá trị hợp lệ nhân 60 để tính giây; kiểm thử tại biên 0, 1, maximum và maximum + 1. |
| GET `/ws/rest/v1/session/{sessionID}` | Dùng `toBaseDto()`: chỉ sao chép id, user, studies, anonymized, sharedSession, requiresShareKey, expiredTime. Không sao chép storeUrl, storeAuth, permissions, shareKey; fixture phải xác định serialization null/field vắng theo cấu hình Java. Phiên không tồn tại trả 400 body rỗng; exception trả 500 body rỗng. |
| PUT `/ws/rest/v1/session` | Body có `id`, `studyInstanceUIDs`. Chỉ lookup cache trực tiếp; không fallback DB trong service update. `studyInstanceUIDs != null` thay studies (kể cả tập rỗng); null không thay đổi. Phiên DB đã được lookup trước đó và warm cache vẫn có thể update cache, không ghi ngược DB. Cache miss trả 200 với kết quả null; cần fixture chốt wire body rỗng/null. Trả DTO đầy đủ từ service, không gọi `toBaseDto()` như GET; không tự đổi contract trong lúc port. |
| POST `/ws/rest/v1/session/{sessionID}/share` | Nguồn không tồn tại/đã shared/hết hạn không tạo phiên, controller trả 400 body rỗng. Phiên mới lưu DB, sharedSession=true, lấy storeUrl/storeAuth/studies từ nguồn và anonymized/requiresShareKey từ request; không sao chép user hoặc permissions. Key khi yêu cầu là chuỗi sáu chữ số, giữ số 0 đầu. Exception trả 500 body rỗng. |
| URL chia sẻ | `usePublicURL=true` trả `PUBLIC_DOMAIN + PATH_PREFIX + id`; trường hợp khác trả `PATH_PREFIX + id`. Hai config ưu tiên bản ghi CONFIG nếu có, fallback cấu hình ứng dụng khi không có. Tái sử dụng PATH_PREFIX/cache Step 8; bổ sung typed TOML cho cấu hình còn thiếu, không tự đọc env. Kiểm thử response theo `SessionShareResponse`, cả share key có/không có. |
| DELETE `/ws/rest/v1/session/{sessionID}` | Xóa cache và DB, thành công trả 200 cùng sessionID. Kiểm chứng gọi lặp với ID không tồn tại và exception 500 body rỗng. |
| Auth và quyền | Tái sử dụng middleware Step 3a. Java filter có ngoại lệ URI kết thúc bằng session ID (GET/DELETE trực tiếp không lookup/share-key tại filter); không suy diễn BR-001 thành kiểm tra key đồng nhất cho mọi route. Kiểm chứng path/query/header `session-key` cùng thứ tự middleware và lỗi validation/controller. |
| Lookup và cleanup | Cache còn hạn được dùng trước; cache hết hạn bị evict và trả missing ngay trong lần gọi đó. Cache miss mới đọc DB, warm cache nếu còn hạn, xóa DB nếu hết hạn. Scheduler chỉ xóa DB với expiredTime < now và khác null, lúc 00:00 Asia/Ho_Chi_Minh; không tự evict toàn bộ cache. |

Nguồn bổ sung: `SessionGetDto.toBaseDto`, `SessionUpdateDto.apply`, `SessionCreateDto.toEntity`, `SessionShareDto`, `SessionShareResponse`, `UrlGeneratorImpl` và `WebRequestFilter`. UC-002 mô tả “phiên ngắn hạn” và share-key ở mức tổng quát; các khác biệt về cache của phiên DB và ngoại lệ auth nêu trên phải có fixture xác nhận trước khi port phần bị ảnh hưởng.

## Việc cần làm

- [x] Đối chiếu source và test Java trong phạm vi tính năng: route/service/job, request/response, permission, lỗi; ghi revision và fixture/expected result suy ra từ source. Đây là đối chiếu contract, không phải kết quả chạy Java runtime.
- [x] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: SESSION và CONFIG; hoàn thiện lưu/chia sẻ/cleanup long-term.
- [x] Session ID extractor từ path, `session` và `sessionId`.
- [x] Short-term lookup trước, PostgreSQL fallback sau.
- [x] Get/update/end session.
- [x] Tạo long-term shared session, anonymous option và share key sáu chữ số.
- [x] Cleanup session hết hạn lúc nửa đêm Asia/Ho_Chi_Minh.
- [x] Tái sử dụng extractor/lookup Step 3a và mở rộng lifecycle; không triển khai lại một auth path độc lập.

## Kiểm tra hoàn thành

- [x] Kiểm thử Rust/PostgreSQL: GET lọc secret, update cache-only, chia sẻ/key/reshare, URL public/config/fallback, delete lặp, lookup DB/cache/expiry và lỗi DB; hồi quy auth/admin/config/tạo phiên Step 8 đạt. Không có migration mới.
- [x] Kiểm thử Rust cho lịch tới nửa đêm UTC+07 và truy vấn cleanup DB với expiry < now; smoke HTTP dev/prod cho GET/share/DELETE đạt. Chưa chạy chờ tới nửa đêm thực tế.
- [ ] Rà soát đầy đủ mapping từng UC-002 main flow, A1..A5, BR-001..BR-007 với case/assertion cụ thể; ghi rõ nhánh đã test, chưa test và khác biệt UC/source. Không suy ra coverage đầy đủ từ việc suite pass.
- [x] Chạy test Java liên quan và lưu kết quả thực tế kèm revision/cấu hình: lượt 2026-10-27 đạt 41 test chung Step 8/9; xem mục bằng chứng bên dưới. Đây là unit/MockMvc với collaborator mock, không phải so sánh runtime Java/Rust.
- [ ] Hoàn tất phạm vi so sánh Java/Rust: bộ 24 fixture Step 9 hiện đạt contract 22/24 (20 khớp sau chuẩn hóa, 2 message lỗi Rust riêng được người dùng chấp nhận), 2 chưa kết luận body 401 do MockMvc sendError. Chuyển kiểu expireIn đã sửa; xem [báo cáo chung](step-08-09-java-rust-comparison.md). Giữ mở để chốt full HTTP container và các phạm vi còn thiếu, bao gồm scheduler.

Các mục source, Rust và test Java đã đạt được đánh dấu riêng. Các mục coverage tổng thể và so sánh cùng fixture Java/Rust còn mở; checklist này không xác nhận toàn bộ lifecycle/scheduler đã tương đương runtime Java.

## Tiến độ

- Trạng thái: `in_review`
- Đã làm: Triển khai GET/PUT/DELETE/share, DTO theo non_null Java, long-term SeaORM dùng schema hiện có, public URL dùng ConfigStore chung, typed TOML và cleanup DB lúc nửa đêm UTC+07 với shutdown hủy task. Bổ sung contract test và smoke HTTP.
- Tiếp theo: Bổ sung phạm vi so sánh còn thiếu và rà soát mapping coverage UC-002; chuyển kiểu expireIn đã sửa, message lỗi Rust riêng được chấp nhận nếu rõ nguyên nhân. Chưa commit hoặc push.
- Vướng mắc: Các khác biệt xác nhận trong bộ fixture đã xử lý theo contract mới. Chưa có Java full-container/DB/cache thật để chốt 2 body 401 và các giới hạn trong báo cáo; chưa khẳng định parity toàn bộ.
- Kiểm tra gần nhất: 2026-10-27 — format, clippy all-targets -D warnings, workspace tests (8 pass; các test DB/JNI skip không tính đạt), PostgreSQL regression riêng (6/6 pass, không skip), debug build và smoke_app.py dev/prod pass, gồm GET/share/DELETE qua TCP. `git diff --check` đạt. PostgreSQL test riêng tại localhost:55439; SESSION/CONFIG cuối lượt test đều 0 bản ghi.
- Cập nhật: 2026-10-27 — Codex; sửa parser expireIn và message rõ nguyên nhân theo yêu cầu người dùng; bộ fixture Step 9 đạt contract 22/24, còn 2 body 401 chưa kết luận. Chưa commit/push.

## Bằng chứng và giới hạn kiểm chứng

- Code: `crates/viewer-app/src/api/session.rs`, `api/session/tests.rs`, auth cache/extractor và task cleanup trong `main.rs`; không có migration mới. Unit test kiểm tra biên thời hạn, key sáu chữ số và thời gian tới nửa đêm (kể cả mili giây). PostgreSQL test kiểm tra GET lọc secret, update cache-only, DB fallback/warm, share options/key/reshare, URL config/fallback, delete lặp, expiry/strict cleanup và lỗi DB trước/sau middleware. Log hồi quy DB: `target/step-09-logs/postgres-regression.log` (không commit).
- Xác minh thêm từ `application.properties`: `spring.jackson.default-property-inclusion=non_null`; GET lược các trường không được sao chép. PUT cache miss trả HTTP 200 body rỗng theo controller trả null. DB DTO chuyển permissions null thành tập rỗng theo `SessionGetDto.from`; short-term DTO Step 8 vẫn giữ snapshot gốc.
- Extractor ưu tiên path, rồi query `session`, rồi `sessionId` cho request context. Auth vẫn kiểm tra path theo WebRequestFilter Java; không thêm query authentication hoặc kiểm tra key cho GET/DELETE terminal. Permission guard hiện có không thay đổi.
- Task cleanup tính nửa đêm theo UTC+07 (Việt Nam không đổi DST), xóa DB với expiry < now, giữ cache; hủy/join task trước khi đóng pool. Đã kiểm tra tính lịch và truy vấn xóa, chưa chạy chờ tới nửa đêm thực tế.
- Java unit/MockMvc đã chạy ngày 2026-10-27: 41 test, 0 failures/errors/skipped, BUILD SUCCESS; cùng revision `c983e4eac389fa6b31f970cfee8f9fce5e20af4c`, Temurin 25.0.4.101 và Maven 3.9.11. Lệnh, danh sách/count 7 lớp và giới hạn ghi tại [kết quả Java chung trong Step 8](step-08-create-session.md#kết-quả-test-java-ngày-2026-10-27-chung-step-89). Log tại `../../../1pacs-viewer-core/target/step-08-09-java-logs/tests.log`, report tại `../../../1pacs-viewer-core/target/surefire-reports/` (không commit). File UrlGeneratorImplTest rỗng, không tính là test URL đạt.
- Đã chạy cùng fixture Java/Rust một phần theo báo cáo chung, chưa đạt parity. Chưa chạy Java integration với PostgreSQL/cache thật, release build/smoke hoặc CI Windows/Ubuntu; không khẳng định các mục này đạt. Scheduler Java test gọi job với repository mock, không kiểm chứng cron nửa đêm. Giữ checkbox parity tổng thể mở.

## Cập nhật contract sau so sánh (2026-10-27)

Share nhận expireIn là số i32 hoặc chuỗi số nguyên (trim/dấu); `"1"` và `1` cùng tạo thời hạn một phút, chuỗi `"0"` theo rule fallback 60 phút. Null/missing/blank string vẫn không đáp ứng @NotNull và trả 400/VALIDATION_ERROR. Chuỗi sai hoặc ngoài i32 bị từ chối trước khi ghi DB. OpenAPI mô tả integer hoặc string.

Theo người dùng, message lỗi không cần giống Spring/Jackson. GET/DELETE và exception DB trong controller giữ behavior đã có. Lỗi decode JSON của PUT/share giữ HTTP 500/INTERNAL_ERROR và envelope `{status,errorCode,message,timestamp}`, nhưng dùng helper chung Step 8/9 chỉ rõ trường/kiểu, JSON object, vị trí cú pháp hoặc Content-Type; không echo input/exception thô. Không đổi lỗi middleware 401 trong lượt này.

Kiểm chứng sau sửa: workspace unit 11 pass, regression PostgreSQL 6/6 pass, export Rust pass; Step 9 contract 22/24 với baseline Java đã lưu, chỉ còn 2 body 401 chưa kết luận. Format/clippy, debug build và smoke HTTP dev/prod đạt. Unit test bao gồm string expiry, i32 bounds, sai kiểu và message không lộ input/map key chứa study ID; ghi số test pass riêng với số fixture contract đạt. Không chạy lại Java vì production/fixture Java không đổi; không chạy release/CI mới.
