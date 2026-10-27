# Step 10 - UC-003 Tra cứu study DICOM

## Tính năng

Chuyển tìm kiếm study và gọi QIDO-RS sang Rust.

**JNI:** Không, trừ normalization có evidence cần dcm4che.

## Nguồn Java

- [module/study/rest/StudyAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/study/rest/StudyAPI.java)
- [module/study/service/StudyServiceImpl.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/study/service/StudyServiceImpl.java)
- [module/qido/service/QidoServiceImpl.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/qido/service/QidoServiceImpl.java)
- [UC-003](../../../1pacs-viewer-core/docs/use_cases/UC-003-tra-cuu-study-dicom.md)

## Việc cần làm

- [x] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [x] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: SESSION đã có; không tạo schema mới nếu không có yêu cầu baseline.
- [x] Validate session và Patient ID.
- [x] Normalize offset/limit chính xác theo business rule.
- [x] Forward QIDO query và store authentication.
- [x] Preserve current behavior của sort parameter.
- [x] Map empty/missing-session response đúng status baseline.

## Kiểm tra hoàn thành

- [x] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [x] UC-003 main flow, A1..A3, BR-001..BR-003; upstream timeout/auth/error.
- [x] Đạt khi: QIDO contract và pagination normalization pass.

## Tiến độ

- Trạng thái: `done`
- Đã làm: POST `/ws/rest/v1/study/search`, session cache/DB dùng chung Step 8–9, validation, QIDO HTTP client, filter/date range, pagination, DTO DICOM JSON và lỗi. Không thêm schema, migration, file output hoặc JNI.
- Tiếp theo: Người dùng đã xác nhận commit riêng Step 10; không tự push hoặc chuyển Step 11.
- Vướng mắc: Không có trong phạm vi đã kiểm chứng. Khác biệt UC-003 A1 được ghi theo code Java bên dưới.
- Kiểm tra gần nhất: 2026-10-27 — Java StudyAPITest 3/3; 8 fixture Java/Rust khớp; workspace unit 15 pass; PostgreSQL regression 7/7 không skip; format, clippy, debug build, smoke dev/prod và live QIDO đạt.
- Cập nhật: 2026-10-27 — Codex; hoàn tất kiểm tra, người dùng đồng ý commit `feat(step-10): implement QIDO study search`.

## Baseline và contract đã xác minh

- Java revision `2373896fca106bab446c0e14bb38f0b8d1725949`.
  [Contract](../baseline/step-10-contract.md), [fixture Java](../baseline/step-10-java.json),
  harness `tools/Step10Baseline.java`; không sửa nguồn hoặc snapshot Java.
- A1 trong UC-003 ghi 204 khi không có study, nhưng Java thực tế trả **200, body []**.
  Giữ code làm baseline; session không tồn tại (A2) trả **204, body rỗng**.
  Upstream 204/không body/JSON lỗi cũng thành 200 với danh sách rỗng.
- sessionID khác null/rỗng; Patient ID khác null, được phép rỗng. Không thêm permission
  hoặc lọc study theo session ngoài hành vi Java.
- Query mặc định 0/100, chuỗi rỗng dùng mặc định. QIDO normalize offset âm về 0,
  limit không dương về 20; response header vẫn giữ offset/limit đầu vào.
  Sai kiểu/ngoài i32 trả 500 INTERNAL_ERROR theo controller/advice Java đã chạy.
  `orderBy` được nhận và bỏ qua.
- QIDO dùng store URL/auth trong session; Authorization không lấy từ caller.
  HTTP client reqwest 0.12.24, TLS rustls, connect 10s/read 60s và follow redirects
  theo Feign Request.Options đã xác minh. Không tự lấy proxy từ environment.
  Lỗi HTTP upstream/mạng/timeout trả 500, message Rust không lộ URL/query/auth/body.
- DTO lấy giá trị đầu, bỏ null/chuỗi rỗng, trim padding DICOM; PN ghép nhóm bằng `=`.
  Fixture kiểm chứng multivalue, PN, JSON sai/rỗng, root object và thiếu VR.

## Bằng chứng kiểm tra ngày 2026-10-27

- `./mvnw.cmd -o -Dtest=StudyAPITest test` tại repo Java với Temurin
  25.0.4.101: BUILD SUCCESS, 3 test, không failure/error/skip.
  Log `target/step-10-logs/java-tests.log` tại repo Rust.
- Harness Java chạy với classpath của report StudyAPITest sau build, xuất
  `target/step-10-logs/baseline-java.json`; so sánh JSON với fixture đã lưu đạt 8/8.
  Log `target/step-10-logs/java-baseline.log` còn ghi validation, 5 trường hợp query,
  DICOM padding/PN rỗng và timeout mặc định Feign. Rust test `recorded_java_contract`
  so sánh query, HTTP status, envelope và DTO; chỉ loại datetime phát sinh.
- `cargo test --workspace --locked`: 15 pass. 9 test DB/JNI/export được đánh dấu
  ignored trong lượt này không tính đạt; PostgreSQL được chạy riêng dưới đây.
  Log `target/step-10-logs/workspace-tests.log`.
- `cargo test -p viewer-app --locked postgres_migrations_transaction_readiness_and_close -- --ignored`:
  7/7 pass, không skip, trên PostgreSQL test riêng localhost:55440.
  Bao gồm Step 10 cache/DB fallback, A1/A2/A3, upstream 401/403/500, query rỗng/sai,
  phân trang/sort, kiểm tra SESSION không đổi và cleanup; hồi quy auth, client,
  permission, config, session Step 8/9. Log `target/step-10-logs/postgres-regression.log`.
  Cuối lượt SESSION=0, CONFIG=0; đã dừng PostgreSQL test và gỡ overlay TOML tạm.
- HTTP store giả qua TCP: URL encoding, filter/auth, không auth, 200/204/JSON lỗi,
  401/403/500, mất kết nối và timeout. Timeout test dùng client timeout 200ms;
  không chờ 60 giây production trong test.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo build --workspace --locked`, `git diff --check`: đạt.
- `python tools/smoke_app.py`: dev/prod, TOML/profile/interpolation/RUST_LOG,
  session/link qua TCP, cấu hình sai/DB sai mật khẩu và shutdown: đạt.
  PostgreSQL tạm ban đầu dùng trust khiến ca mật khẩu sai không thể fail startup;
  đã đổi riêng cluster test sang SCRAM, chạy lại toàn smoke đạt; không sửa smoke.
- Live store do người dùng cung cấp: QIDO trực tiếp 200/1 study; tạo session Rust
  200; tìm study qua Rust 200/1 study; DTO khớp metadata từ upstream; không có kết quả,
  phân trang và tạo link đạt. Session test đã xóa, ứng dụng shutdown sạch. Không lưu
  credentials, Study UID hoặc dữ liệu bệnh nhân vào fixture/tài liệu.

Giới hạn: baseline Java dùng service/controller/dcm4che thật với collaborator mock,
không chạy Java full-container với store thật. Live test dùng Rust debug, chưa thử
frontend hiển thị pixel, release build hoặc CI Windows/Ubuntu trong lượt này.
WADO/pixel retrieval thuộc Step 11; Step 10 hoàn tất không có nghĩa toàn bộ Viewer
đã xem ảnh qua Rust. Log/raw artifact nằm trong target, không commit.
