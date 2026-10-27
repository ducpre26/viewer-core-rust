# Step 8 - UC-001 Khởi tạo phiên xem ảnh 2D

> Refactor TOML ngày 2026-10-26: module nhận typed config từ registry Summer;
> giữ app.toml/profile/interpolation/RUST_LOG theo xác nhận người dùng.
> Checklist/evidence bên dưới là lịch sử step; kết quả refactor mới nằm trong
> [mục cấu hình](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#toml-config).

## Tính năng

Chuyển API tạo session 2D và URL Viewer sang Rust.

**JNI:** Không.

## Nguồn Java

- [client/ClientAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/client/ClientAPI.java)
- [module/session/service/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/session/service/)
- [UC-001](../../../1pacs-viewer-core/docs/use_cases/UC-001-khoi-tao-phien-xem-anh-2d.md)

## Việc cần làm

- [x] Đối chiếu source và test Java trong phạm vi tính năng: route/service, request/response, permission, lỗi; ghi revision và fixture/expected result suy ra từ source. Đây là đối chiếu contract, không phải kết quả chạy Java runtime.
- [x] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: CLIENT, permission, CONFIG và SESSION; tái sử dụng repository đã có, hoàn thiện phần tạo session.
- [x] Basic Auth cho `/client/**` và IP allow-list.
- [x] Validate store/studies.
- [x] Snapshot permission tại thời điểm tạo session.
- [x] Normalize expiry theo config.
- [x] Tạo short-term session và viewer URL.
- [x] Tái sử dụng auth/session foundation Step 3a; không tạo cơ chế credential/permission thứ hai.

## Kiểm tra hoàn thành

- [x] Kiểm thử Rust theo fixture Rust hiện có đã đạt; không migration mới, hồi quy Step 3a/4/5 đạt. Đây là kết quả suite Rust, không xác nhận mọi response khớp Java; lượt so sánh 2026-10-27 phát hiện khác biệt ghi ở báo cáo bên dưới. Giới hạn WADO prefetch ghi bên dưới.
- [x] Kiểm thử Rust cho UC-001 main flow, A1, A2, BR-001..BR-003; credential/IP/expiry/permission snapshot; BR-001 theo validation Java đã được người dùng xác nhận.
- [x] Kiểm chứng postcondition phía Rust: session/link/permission/database-cache pass.
- [x] Chạy test Java liên quan và lưu kết quả thực tế kèm revision/cấu hình: lượt 2026-10-27 đạt 41 test chung Step 8/9; xem bằng chứng bên dưới. Đây là unit/MockMvc với collaborator mock, không phải so sánh runtime Java/Rust.
- [ ] Hoàn tất phạm vi so sánh Java/Rust: bộ 12 fixture Step 8 hiện đạt contract 12/12 (10 khớp sau chuẩn hóa, 2 message lỗi Rust riêng được người dùng chấp nhận); đã sửa field null và chuyển kiểu expireIn. Xem [báo cáo chung](step-08-09-java-rust-comparison.md). Giữ mục tổng thể mở vì chưa kiểm chứng cùng fixture Basic/IP/permission snapshot và Java DB/cache Spring thật.

Các mục Rust đã đạt được giữ nguyên. Test Java liên quan đã chạy đạt; mục so sánh cùng fixture Java/Rust còn mở. Checklist này không xác nhận tương đương runtime Java/Rust.

## Tiến độ implementation trước quyết định TOML

- Trạng thái lịch sử: `done` — implementation và kiểm tra Rust Windows local đạt; không bao gồm xác nhận tương đương runtime Java/Rust. Test Java liên quan đã đạt ngày 2026-10-27; so sánh cùng fixture còn mở, CI Windows/Ubuntu chưa chạy.
- Đã làm: Port POST `/client/link` và `/client/getlink`; dùng Basic/IP/cache/lookup/guard Step 3a, repository client-permission Step 4 và ConfigStore Step 5. Tạo UUID v4, cache-only session, snapshot quyền hiện tại, expiry phút/UTC milliseconds và URL với CONFIG override. CachedSession giữ nguyên study set (null/blank/comma/dedup), không thay schema. ConfigStore được chia sẻ giữa API config và tạo link để invalidation có hiệu lực ngay. Thêm cấu hình môi trường, tài liệu, contract test PostgreSQL và smoke HTTP thật.
- Tiếp theo: Bổ sung các phạm vi so sánh còn thiếu; ba khác biệt field null/chuyển kiểu expireIn Step 8/9 đã sửa. Message lỗi không phải khớp nguyên văn Java theo quyết định người dùng, nhưng phải chỉ rõ nguyên nhân. Không tự commit/push; lịch sử `done` ở trên phản ánh implementation/test trước lượt so sánh.
- Giới hạn: Bộ 12 fixture Step 8 đạt contract sau sửa; chưa khẳng định parity toàn bộ. Java dùng MockMvc và storage double, Rust dùng PostgreSQL thật; chưa có Java integration với DB/cache Spring thật hoặc CI Windows/Ubuntu. WADO metadata prefetch để Step 11; không ảnh hưởng kết quả API/cache của Step 8.
- Kiểm tra thực tế (2026-10-26): `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` đạt (5 unit; các test cần DB/JNI không tính lượt skip). Lệnh PostgreSQL trong DEVELOPMENT_COMMANDS chạy đủ 5 test, không skip, gồm Step 3a/4/5/8 và readiness/migration. `cargo build --workspace --locked` và `cargo build --workspace --release --locked` đạt; `python tools/smoke_app.py` và `python tools/smoke_app.py --release` đạt dev/prod, cả hai API tạo link/session qua TCP, lookup cache, OpenAPI, startup fail-safe, logging và shutdown.
- Hồi quy JNI: `python tools/smoke_jni.py --java-home 'C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot' --app` đạt: subprocess suite, PostgreSQL/JVM readiness, HTTP dev/prod với tạo link/session, shutdown và fatal recovery. Không đổi adapter/facade; không chạy lại JNI release suite. Release HTTP/JNI-off đã chạy riêng đạt.
- Evidence: `target/step-08-logs/`: `workspace-tests.log`, `postgres-regression.log`, `format.log`, `build-debug.log`, `build-release.log`, `smoke-debug.log`, `smoke-release.log`, `jni-regression.log`, `final-database-state.log`, `postgres-stop.log`. PostgreSQL 18 tạm localhost đã dừng/xóa; trạng thái cuối CLIENT/SESSION/CONFIG/CLIENT_PERMISSION đều 0 bản ghi, 0 kết nối ứng dụng. Log không commit. Smoke ban đầu thất bại vì DB tạm dùng trust làm case mật khẩu sai không bị từ chối; đã cấu hình SCRAM và chạy lại đạt. Sửa đọc output smoke theo UTF-8 để tránh lỗi decode Windows.
- Cập nhật: 2026-10-27 — Codex; sửa field null/chuyển kiểu expireIn và message rõ nguyên nhân theo yêu cầu người dùng; bộ 12 fixture Step 8 đạt contract mới. Giữ lịch sử người dùng xác nhận commit Step 8 ngày 2026-10-26; lượt sửa này chưa commit/push.

## Kết quả test Java ngày 2026-10-27 (chung Step 8/9)

- Revision Java: `c983e4eac389fa6b31f970cfee8f9fce5e20af4c`; JDK Temurin 25.0.4.101, Maven 3.9.11; không profile ứng dụng/DB thật, test dùng Mockito và MockMvc standalone. Không sửa source/test Java để chạy.
- Lệnh từ repo Java: `mvn test -Dtest=ClientAPITest,ShortTermSessionServiceTest,SessionServiceImplTest,SessionAPITest,SessionSchedulerTest,UrlGeneratorImplTest,SessionPermissionEvaluatorTest,WebRequestFilterMdcTest -Dmaven.repo.local=C:/Users/Admin/.m2/repository -B -ntp`.
- BUILD SUCCESS: 41 test, 0 failures, 0 errors, 0 skipped; 7 lớp thực thi: ClientAPITest (3), ShortTermSessionServiceTest (5), SessionServiceImplTest (14), SessionAPITest (6), SessionSchedulerTest (3), SessionPermissionEvaluatorTest (6), WebRequestFilterMdcTest (4). Đây là một lượt test dùng chung, không cộng thành 82 test cho hai step. `UrlGeneratorImplTest.java` rỗng nên không có test URL được thực thi.
- Log: `../../../1pacs-viewer-core/target/step-08-09-java-logs/tests.log`; XML/text report tại `../../../1pacs-viewer-core/target/surefire-reports/` (artifact local, không commit). Lần đầu sandbox chặn ghi metadata cache; chạy lại với quyền truy cập cache được duyệt đã đạt. Không cần đổi cấu hình dự án hoặc cung cấp credential mới.
- Lượt 41 test trên chưa chạy Java với PostgreSQL/cache Spring thật và không phải so sánh cùng fixture. Lượt so sánh bổ sung sau đó được ghi riêng tại [báo cáo chung](step-08-09-java-rust-comparison.md); chưa chứng minh cron chạy đúng nửa đêm hoặc toàn bộ nhánh share/URL/validation.

## Contract đối chiếu từ source Java

Java revision: `c983e4eac389fa6b31f970cfee8f9fce5e20af4c`. Khi port chỉ đọc mã và test Java (ClientAPITest, ShortTermSessionServiceTest, SessionServiceImplTest); test Java được chạy bổ sung ngày 2026-10-27 như mục trên. Bảng dưới vẫn là expected result từ source để kiểm thử Rust, không phải bộ output thu được từ lượt so sánh Java/Rust.

| Fixture tổng hợp / nhánh | Expected result từ Java |
|---|---|
| POST `/ws/rest/client/link`, storeUrl=`http://store.invalid`, studies=[`1.2.3`], user=`fixture-user` | 200 `{link,session}`; UUID v4; trả DTO session đầy đủ, không dùng toBaseDto, các boolean false. Các field null (shareKey, permissions/storeAuth khi thiếu) bị lược theo non_null; Rust đã sửa và kiểm chứng theo bộ fixture chung. |
| Basic sai/thiếu, client thiếu, BCrypt sai, IP không match | 401 trước handler; dùng middleware Step 3a. Security disabled bỏ Basic, principal null. Admin không tự được permission ADMIN. |
| `/link` storeUrl hoặc studies thiếu/null | 400 VALIDATION_ERROR; studies=[] → 400 body rỗng. StoreUrl rỗng, studies chứa blank/null vẫn được nhận. |
| `/getlink` studyUIDs thiếu/null/rỗng | 400 validation; nested Store không có @Valid nên thiếu store/url vẫn tạo được session. |
| `/getlink` studyUIDs={`1.2.3`:{url:`http://store.invalid`,authentication:`fixture`}}, userID | 200 envelope header.code=200/message=`Viewer session created: <id>`, body là URL. Tập key là studies; store cuối có url khác null được dùng, thứ tự HashMap không là contract. |
| expireIn thiếu/null/0/âm/>default | Dùng viewer.default-expire-minus (60 phút); 1..default giữ nguyên. Thời gian mili giây UTC. |
| expireIn là integer string | Cả link/getlink nhận số i32 hoặc chuỗi số nguyên có trim/dấu, ví dụ `"1"`, `" +2 "`; chuỗi rỗng/blank thành null rồi dùng default như cũ. Chuỗi sai, bool/array hoặc ngoài i32 bị từ chối; không đưa giá trị input vào message lỗi. Không khẳng định hỗ trợ mọi phép coercion Jackson ngoài phạm vi này. |
| Client có mapping MODIFY_OVERLAY_CONFIG, sau đó bỏ mapping | Phiên cũ giữ snapshot, phiên mới đọc mapping hiện tại. Security disabled/không principal → permissions null; client không có mapping → []. Basic sysadmin không có CLIENT row → 404 NOT_FOUND từ IBaseService.findByIdInternal, dù comment ClientAPI nói bỏ qua. Có CLIENT row → đọc mapping như client thường, không tự cấp ADMIN. |
| CONFIG PATH_PREFIX tồn tại/đổi/xóa | Ưu tiên value CONFIG kể cả chuỗi rỗng; dùng cache ConfigStore Step 5 và invalidation chung; thiếu → viewer.path-prefix (`/viewer?session=`). |
| Tạo phiên thành công | Chỉ cache session Step 3a (TTL 120 phút/capacity 1000), không INSERT SESSION, không file; lookup/permission guard dùng được ngay. |

Khác biệt UC-001 BR-001: người dùng xác nhận giữ validation Java cho cả hai API (2026-10-26). Không siết blank store/nested store.

Dependency ngoài phạm vi: Java gọi best-effort async WADO metadata prefetch sau tạo session; lỗi không đổi kết quả tạo phiên. WADO/series metadata thuộc Step 11, chưa có trong Rust và không thực hiện ở Step 8. Không port seal/JNI. Không có migration mới: CLIENT, CLIENT_PERMISSION, CONFIG, SESSION đã có ở Step 3a/4/5.

## Cập nhật contract sau so sánh (2026-10-27)

Theo yêu cầu người dùng, đã sửa null fields của response tạo phiên và parser expireIn dùng chung Step 8/9; giữ null trong cache, chỉ lược field null khi xuất DTO JSON. OpenAPI mô tả expireIn là integer hoặc string. Không đổi business rule normalize expiry.

Message lỗi JSON/body được phép khác Java. Rust giữ HTTP 500/INTERNAL_ERROR và envelope hiện có, thay `Invalid request body` bằng mô tả trường/kiểu cần gửi, JSON object, vị trí lỗi cú pháp hoặc Content-Type. Test kiểm tra không echo giá trị input. Validation thiếu field vẫn 400/VALIDATION_ERROR. Không áp dụng thay đổi helper này cho API admin/config ngoài Step 8/9.

Kiểm chứng sau sửa: workspace unit 11 pass; regression PostgreSQL 6/6 pass; export fixture Rust pass và so sánh Step 8 contract 12/12 với baseline Java đã lưu; format/clippy, debug build và smoke HTTP dev/prod đạt. Bốn message khác Java được comparator kiểm tra prefix có ý nghĩa và giữ nguyên status/errorCode/toàn bộ phần còn lại, không bỏ qua mọi message lỗi một cách chung chung. Không chạy lại Java vì production/fixture Java không đổi; không chạy release/CI mới.
