# So sánh Step 8/9 Rust với Java — 2026-10-27

## Kết quả hiện tại sau sửa theo yêu cầu người dùng

Ba khác biệt field null/chuyển kiểu expireIn đã sửa. Message lỗi được phép khác Java theo quyết định người dùng ngày 2026-10-27; Rust phải mô tả nguyên nhân rõ và không echo input. Đã chạy lại export Rust trên cùng 36 fixture và so sánh với baseline Java đã lưu (không đổi Java production code/fixture).

| Phạm vi | Fixture | Khớp sau chuẩn hóa | Message riêng đạt contract | Chưa kết luận body 401 |
|---|---:|---:|---:|---:|
| Step 8 | 12 | 10 | 2 | 0 |
| Step 9 | 24 | 20 | 2 | 2 |
| Tổng | 36 | 30 | 4 | 2 |

**Step 8 đạt contract 12/12; Step 9 đạt 22/24 trong bộ fixture này.** Không còn khác biệt xác nhận ngoài những message Rust riêng đã được chấp nhận; chưa khẳng định parity toàn hệ thống. Comparator vẫn trả exit code 1 do hai body 401 chưa kết luận, không coi chúng đã đạt.

Response tạo session lược field null, giữ false/empty array/empty string và null ở cấp phần tử study. Cache/snapshot permission không đổi. Cả link/getlink/share dùng parser expireIn chung nhận i32 hoặc integer string (trim/dấu; blank thành null), reject sai kiểu/ngoài i32 và cập nhật OpenAPI. Business rule default/fallback vẫn giữ nguyên.

Rust giữ HTTP 500/INTERNAL_ERROR và envelope `{status,errorCode,message,timestamp}` cho lỗi decode JSON/body của các route Step 8/9; message ví dụ:

- `Field 'expireIn' must be a 32-bit integer or integer string, or null`
- `Field 'studies' must be an array of strings (entries may be null), or null`
- `Request body must be a JSON object with valid field types`
- `Malformed JSON at line 1, column 1`
- `Content-Type must be application/json`

Chỉ nêu tên field DTO đã biết; không đưa giá trị input hoặc study ID trong map key vào message. Không trả exception/stacktrace thô. Field bắt buộc thiếu/null vẫn đi theo 400/VALIDATION_ERROR. Không mở rộng helper sang API admin/config ở các step trước.

Comparator giữ cả `match` (khớp sau chuẩn hóa như trước), `contractMatch` và `acceptedMessage`; chỉ chấp nhận 4 fixture message lỗi đã định danh khi prefix message rõ nguyên nhân và tất cả field khác (status/errorCode/DB/body còn lại) vẫn khớp. Không bỏ qua message trên mọi lỗi. Unit test bổ sung kiểm tra sai kiểu/overflow/string expiry và không echo input. Workspace 11 unit pass, PostgreSQL regression 6/6 pass, export baseline Rust pass; format/clippy đạt. Build/smoke sau sửa được ghi ở tiến độ Step 8/9.

## Kết quả trước sửa (lịch sử)

Ở lượt so sánh ban đầu, chưa đạt tương đương API trong phạm vi fixture đã chạy; lượt đó không thay đổi production code. Các số liệu và khác biệt bên dưới là lịch sử trước khi sửa, không phải lỗi còn tồn tại hiện tại.

| Phạm vi | Fixture | Khớp sau chuẩn hóa | Khác biệt xác nhận | Chưa kết luận body do MockMvc sendError |
|---|---:|---:|---:|---:|
| Step 8 | 12 | 6 | 6 | 0 |
| Step 9 | 24 | 19 | 3 | 2 |
| Tổng | 36 | 25 | 9 | 2 |

Hai export test đều pass nghĩa là xuất được kết quả, không phải parity pass. Script so sánh trả exit code 1 vì còn khác biệt. Đây là 36 trường hợp cụ thể, không phải tỷ lệ coverage toàn bộ UC-001/UC-002.

## Cách kiểm chứng và giới hạn

- Java production revision `c983e4eac389fa6b31f970cfee8f9fce5e20af4c`, thêm test chưa commit `src/test/java/vn/saolasoft/viewer/baseline/Step0809BaselineTest.java`; Temurin 25.0.4.101, Maven 3.9.11. Test gọi controller ClientAPI/SessionAPI, SessionServiceImpl, ShortTermSessionService.create, UrlGeneratorImpl và WebRequestFilter thật qua MockMvc. Short/long-term storage là double in-memory; UUID khi lưu và lỗi cache key null được mô phỏng theo JPA/Spring, không phải kiểm chứng Spring cache proxy hoặc PostgreSQL Java.
- Converter Java dùng Jackson 3, cấu hình non_null, ISO Instant, timezone và date-format đối chiếu application.properties. Đây là cấu hình converter tường minh trong test, chưa khởi động full Boot context để xác minh toàn bộ auto-configuration.
- Rust chạy working tree hiện tại qua router/auth/handler thật và PostgreSQL 18 riêng localhost:55439, schema migration thật. Mỗi fixture bắt đầu với cùng ba phiên cache fixture; không thay dữ liệu production. Basic security bị tắt ở cả hai phía; session path/share-key vẫn qua filter. Không kiểm chứng Basic/IP/permission snapshot bằng cùng fixture trong lượt này.
- Input chung: `docs/fixtures/step-08-09-http.json`. So sánh HTTP status/body, số phiên lưu, studies CSV và snapshot DB của shared session: store URL/auth, anonymized/shared, có/không user/permissions, expiry.
- Chuẩn hóa UUID và share key ngẫu nhiên (chỉ sau kiểm tra key là sáu chữ số), timestamp sinh tự động; expiry mới tính thành số phút còn lại làm tròn để so semantics thời hạn. Sắp xếp study/permission set và thứ tự thông báo nhiều lỗi Bean Validation. Không lược trường null, không sửa status hoặc message lỗi khác khi so sánh. Không kiểm chứng độ chính xác expiry dưới một phút hoặc cách viết phần lẻ giây trên wire.
- Chưa chạy cùng fixture cho DB lỗi, transaction/rollback, cleanup, cron nửa đêm, cache TTL/concurrency, Basic/IP, config invalidation hoặc session hết hạn. Kết quả test riêng trước đó không được tính thành cùng-fixture parity của các phần này.

## Khác biệt ban đầu và cách xử lý

| Nhóm | Fixture | Java | Rust | Mức ảnh hưởng |
|---|---|---|---|---|
| Step 8: trường null | link-minimal, link-full, link-blank-store | Lược các field null theo non_null, ví dụ permissions/shareKey và user/storeAuth khi thiếu | Trước sửa emit field null | Đã sửa lược field null, fixture khớp |
| Step 8: chuyển kiểu expireIn | link-coerce-integer: `expireIn: "1"` | 200, nhận một phút | Trước sửa trả 500, không tạo phiên | Đã sửa parser chung, fixture khớp |
| Step 9: chuyển kiểu expireIn | share-wrong-type: `expireIn: "1"` | 200, lưu một phiên chia sẻ | Trước sửa trả 500, không lưu phiên | Đã sửa parser chung, fixture khớp cả tác động DB |
| Step 8/9: message lỗi JSON/body | link-wrong-type, link-null-body, update-null-body, share-null-body | 500 INTERNAL_ERROR với message từ Spring/Jackson | Trước sửa message chung `Invalid request body` | Đã thay bằng message rõ nguyên nhân; khác Java được người dùng chấp nhận, status/errorCode giữ nguyên |

Ví dụ response `/client/link` tối thiểu sau chuẩn hóa, phần session Java:

```json
{"id":"<uuid>","storeUrl":"http://store.invalid","studies":["a"],"anonymized":false,"sharedSession":false,"requiresShareKey":false,"expiredTime":{"ttlMinutes":60}}
```

Trước sửa Rust có thêm `permissions: null`, `shareKey: null`, `storeAuth: null`, `user: null` mà Java lược bỏ. Sau sửa các field này được lược như Java.

Vị trí Rust sau sửa: `crates/viewer-app/src/api/client.rs` (SessionCreator.create, LinkInput/LegacyInput), `crates/viewer-app/src/api/session.rs` (Share và update/share), `crates/viewer-app/src/api/request.rs` (parser và message lỗi dùng chung).

## Các nhánh khớp trong fixture

- Step 8: validation studies thiếu/rỗng; legacy getlink hợp lệ, thiếu/rỗng studyUIDs và nested store null. Đối với legacy thiếu studyUIDs, thứ tự hai validation message không là contract và đã chuẩn hóa.
- Step 9: GET phiên tồn tại/không tồn tại/shared và DTO lọc; PUT replace/no-change/empty/missing/null-id (null-id phụ thuộc mô phỏng cache-key như giới hạn trên); share valid, fallback 60 phút, vượt giới hạn, biên 129600 phút, key/anonymized, public URL, expireIn bắt buộc, từ chối reshare; DELETE tồn tại/missing.
- Sáu fixture share thành công với input đúng kiểu có snapshot DB tương ứng khớp: giữ store URL/auth, không copy user/permissions, expiry 1/60/129600 phút, anonymized và shared flag đúng. Java sử dụng storage double nên chưa phải bằng chứng persistence Java với DB thật.

## Hai trường hợp chưa kết luận response body

`share-missing` và `share-baseline-key` không cung cấp key: cả hai phía trả 401 và không ghi DB. Java filter gọi `sendError`; MockMvc không chạy error dispatch của servlet container nên body thu được rỗng, Rust trả `Access is denied`. Chưa coi đây là production body mismatch đã xác nhận; cần chạy Java full HTTP container để chốt body thực tế.

## Tái chạy và artifact

Từ repo Java:

```powershell
mvn test '-Dtest=Step0809BaselineTest' '-Dmaven.repo.local=C:/Users/Admin/.m2/repository' -B -ntp
```

Từ repo Rust, với PostgreSQL test riêng được cấu hình trong TOML và bảng SESSION rỗng:

```powershell
cargo test -p viewer-app --locked step_08_09_baseline_export -- --ignored
python tools/compare_step_08_09_baseline.py
```

- Java input dùng file fixture tại checkout Rust bên cạnh; cần giữ hai repo cùng thư mục cha. Cache Maven có thể đổi theo môi trường.
- Raw Java: `../1pacs-viewer-core/target/step-08-09-java-logs/baseline-java.json`, log `baseline-run.log` cùng thư mục.
- Raw Rust: `target/step-09-logs/baseline-rust.json`, `baseline-export.log`.
- Kết quả normalized từng fixture: `target/step-09-logs/comparison.json`, tóm tắt `comparison-summary.log`.
- Artifact target là local, không commit. Fixture/harness/script được giữ để tái chạy. `cargo fmt` và `cargo clippy --workspace --all-targets --locked -- -D warnings` đạt sau bổ sung test. Không sửa production code trong lượt này.
