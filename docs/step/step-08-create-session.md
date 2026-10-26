# Step 8 - UC-001 Khởi tạo phiên xem ảnh 2D

## Tính năng

Chuyển API tạo session 2D và URL Viewer sang Rust.

**JNI:** Không.

## Nguồn Java

- [client/ClientAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/client/ClientAPI.java)
- [module/session/service/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/session/service/)
- [UC-001](../../../1pacs-viewer-core/docs/use_cases/UC-001-khoi-tao-phien-xem-anh-2d.md)

## Việc cần làm

- [x] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [x] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: CLIENT, permission, CONFIG và SESSION; tái sử dụng repository đã có, hoàn thiện phần tạo session.
- [x] Basic Auth cho `/client/**` và IP allow-list.
- [x] Validate store/studies.
- [x] Snapshot permission tại thời điểm tạo session.
- [x] Normalize expiry theo config.
- [x] Tạo short-term session và viewer URL.
- [x] Tái sử dụng auth/session foundation Step 3a; không tạo cơ chế credential/permission thứ hai.

## Kiểm tra hoàn thành

- [x] API/output và tác động DB/file khớp baseline trong phạm vi tạo link/session; không migration mới, hồi quy Step 3a/4/5 đạt. Giới hạn WADO prefetch ghi bên dưới.
- [x] UC-001 main flow, A1, A2, BR-001..BR-003; credential/IP/expiry/permission snapshot; BR-001 theo validation Java đã được người dùng xác nhận.
- [x] Đạt khi: session/link/permission/database-cache postcondition pass.

## Tiến độ

- Trạng thái: `done` — implementation và kiểm tra Windows local đạt; CI Windows/Ubuntu chưa chạy.
- Đã làm: Port POST `/client/link` và `/client/getlink`; dùng Basic/IP/cache/lookup/guard Step 3a, repository client-permission Step 4 và ConfigStore Step 5. Tạo UUID v4, cache-only session, snapshot quyền hiện tại, expiry phút/UTC milliseconds và URL với CONFIG override. CachedSession giữ nguyên study set (null/blank/comma/dedup), không thay schema. ConfigStore được chia sẻ giữa API config và tạo link để invalidation có hiệu lực ngay. Thêm cấu hình môi trường, tài liệu, contract test PostgreSQL và smoke HTTP thật.
- Tiếp theo: Chờ yêu cầu tiếp theo của người dùng; không tự push hoặc chuyển Step 9.
- Giới hạn: Không có baseline Java runtime hoặc kết quả CI Windows/Ubuntu. WADO metadata prefetch để Step 11; không ảnh hưởng kết quả API/cache của Step 8.
- Kiểm tra thực tế (2026-10-26): `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` đạt (5 unit; các test cần DB/JNI không tính lượt skip). Lệnh PostgreSQL trong DEVELOPMENT_COMMANDS chạy đủ 5 test, không skip, gồm Step 3a/4/5/8 và readiness/migration. `cargo build --workspace --locked` và `cargo build --workspace --release --locked` đạt; `python tools/smoke_app.py` và `python tools/smoke_app.py --release` đạt dev/prod, cả hai API tạo link/session qua TCP, lookup cache, OpenAPI, startup fail-safe, logging và shutdown.
- Hồi quy JNI: `python tools/smoke_jni.py --java-home 'C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot' --app` đạt: subprocess suite, PostgreSQL/JVM readiness, HTTP dev/prod với tạo link/session, shutdown và fatal recovery. Không đổi adapter/facade; không chạy lại JNI release suite. Release HTTP/JNI-off đã chạy riêng đạt.
- Evidence: `target/step-08-logs/`: `workspace-tests.log`, `postgres-regression.log`, `format.log`, `build-debug.log`, `build-release.log`, `smoke-debug.log`, `smoke-release.log`, `jni-regression.log`, `final-database-state.log`, `postgres-stop.log`. PostgreSQL 18 tạm localhost đã dừng/xóa; trạng thái cuối CLIENT/SESSION/CONFIG/CLIENT_PERMISSION đều 0 bản ghi, 0 kết nối ứng dụng. Log không commit. Smoke ban đầu thất bại vì DB tạm dùng trust làm case mật khẩu sai không bị từ chối; đã cấu hình SCRAM và chạy lại đạt. Sửa đọc output smoke theo UTF-8 để tránh lỗi decode Windows.
- Cập nhật: 2026-10-26 — Codex; người dùng đã xác nhận commit Step 8, không push.

## Baseline trước khi port

Java revision: `c983e4eac389fa6b31f970cfee8f9fce5e20af4c`. Chỉ đọc mã và test Java (ClientAPITest, ShortTermSessionServiceTest, SessionServiceImplTest), không chạy baseline Java.

| Fixture tổng hợp / nhánh | Expected result từ Java |
|---|---|
| POST `/ws/rest/client/link`, storeUrl=`http://store.invalid`, studies=[`1.2.3`], user=`fixture-user` | 200 `{link,session}`; UUID v4; session đầy đủ storeUrl/storeAuth/user/studies/permissions/expiredTime; các boolean false, shareKey null. Không dùng toBaseDto. |
| Basic sai/thiếu, client thiếu, BCrypt sai, IP không match | 401 trước handler; dùng middleware Step 3a. Security disabled bỏ Basic, principal null. Admin không tự được permission ADMIN. |
| `/link` storeUrl hoặc studies thiếu/null | 400 VALIDATION_ERROR; studies=[] → 400 body rỗng. StoreUrl rỗng, studies chứa blank/null vẫn được nhận. |
| `/getlink` studyUIDs thiếu/null/rỗng | 400 validation; nested Store không có @Valid nên thiếu store/url vẫn tạo được session. |
| `/getlink` studyUIDs={`1.2.3`:{url:`http://store.invalid`,authentication:`fixture`}}, userID | 200 envelope header.code=200/message=`Viewer session created: <id>`, body là URL. Tập key là studies; store cuối có url khác null được dùng, thứ tự HashMap không là contract. |
| expireIn thiếu/null/0/âm/>default | Dùng viewer.default-expire-minus (60 phút); 1..default giữ nguyên. Thời gian mili giây UTC. |
| Client có mapping MODIFY_OVERLAY_CONFIG, sau đó bỏ mapping | Phiên cũ giữ snapshot, phiên mới đọc mapping hiện tại. Security disabled/không principal → permissions null; client không có mapping → []. Basic sysadmin không có CLIENT row → 404 NOT_FOUND từ IBaseService.findByIdInternal, dù comment ClientAPI nói bỏ qua. Có CLIENT row → đọc mapping như client thường, không tự cấp ADMIN. |
| CONFIG PATH_PREFIX tồn tại/đổi/xóa | Ưu tiên value CONFIG kể cả chuỗi rỗng; dùng cache ConfigStore Step 5 và invalidation chung; thiếu → viewer.path-prefix (`/viewer?session=`). |
| Tạo phiên thành công | Chỉ cache session Step 3a (TTL 120 phút/capacity 1000), không INSERT SESSION, không file; lookup/permission guard dùng được ngay. |

Khác biệt UC-001 BR-001: người dùng xác nhận giữ validation Java cho cả hai API (2026-10-26). Không siết blank store/nested store.

Dependency ngoài phạm vi: Java gọi best-effort async WADO metadata prefetch sau tạo session; lỗi không đổi kết quả tạo phiên. WADO/series metadata thuộc Step 11, chưa có trong Rust và không thực hiện ở Step 8. Không port seal/JNI. Không có migration mới: CLIENT, CLIENT_PERMISSION, CONFIG, SESSION đã có ở Step 3a/4/5.
