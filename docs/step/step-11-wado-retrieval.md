# Step 11 - UC-004 Truy xuất dữ liệu ảnh DICOM

## Tính năng

Chuyển proxy WADO-RS/WADO-URI và kiểm tra phạm vi study sang Rust.

**JNI:** Không cần trong phạm vi đã chốt; payload/metadata đi nguyên byte qua Rust.

**Quyết định người dùng 2026-10-27:** Bỏ nhánh key image cục bộ ở step này;
giữ lỗi study ngoài phạm vi giống Java; chưa prefetch metadata, quay lại test
Step 8 sau. Vì vậy không triển khai STUDY_KEY_IMAGE/file/JNI cho key image ở đây.

## Nguồn Java

- [module/wado/rest/WadoRsAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/wado/rest/WadoRsAPI.java)
- [module/wado/service/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/wado/service/)
- [UC-004](../../../1pacs-viewer-core/docs/use_cases/UC-004-truy-xuat-du-lieu-anh-dicom.md)

## Việc cần làm

- [x] Facade/JNI: không áp dụng cho proxy thuần theo phạm vi người dùng đã chốt; không bổ sung operation hoặc call JNI.
- [x] Đối chiếu route/service/test/filter Java; revision, fixture và expected result tại [contract](../baseline/step-11-contract.md) và [31 fixture](../baseline/step-11-java.json).
- [x] Tái sử dụng SESSION/cache/SeaORM hiện có; không migration hoặc schema mới.
- [x] Enforce study thuộc session; study ngoài scope trả 500 rỗng như Java; session thiếu/hết hạn/share-key sai trả 401 qua filter.
- [x] Proxy study/series/instance/frame/thumbnail/metadata/bulk data và WADO-URI; không nhánh key image cục bộ.
- [x] Preserve status, content type, transfer syntax và safe response headers; không forward credential của caller hoặc upstream cookie/CORS/transport header.
- [x] Streaming/backpressure và client disconnect: pull-based body, không buffer toàn bộ payload.
- [x] DICOM JSON parse/normalize: không áp dụng trong phạm vi proxy đã chốt; metadata giữ nguyên byte.

## Kiểm tra hoàn thành

- [x] JNI/codec: không áp dụng vì không có operation/call trong phạm vi này; cancellation/timeout của HTTP stream được kiểm tra riêng.
- [x] API/output khớp fixture Java trong phạm vi đã chốt; SESSION không đổi sau DB fallback, dữ liệu test đã dọn; không ghi file DICOM hoặc thêm migration. Hồi quy workspace và auth/session qua WADO đạt.
- [x] UC-004 main flow, A1, A2, BR-001, BR-002; 16 MiB integrity, slow client, backpressure, disconnect, upstream error/truncated body, read timeout và lỗi kết nối.
- [x] WADO streaming/media contract pass; byte stream trả ngay khi upstream chưa hoàn tất, không buffer toàn bộ payload.

## Tiến độ

- Trạng thái: `done` (phạm vi đã chốt).
- Đã làm: 14 route WADO-RS + WADO-URI, scope/auth/share-key, proxy request/status/header và streaming; fixture Java, Rust TCP/DB tests, runtime Postman smoke và tài liệu chạy lại.
- Commit: Người dùng đã xác nhận tạo commit riêng Step 11 ngày 2026-10-27. Key image và prefetch không thuộc phần hoàn thành này; prefetch quay lại kiểm thử Step 8 sau theo yêu cầu.
- Vướng mắc: Không trong phạm vi đã chốt.
- Cập nhật: 2026-10-27 — Codex triển khai và kiểm chứng bằng DB cấu hình hiện tại theo yêu cầu người dùng; không dùng cluster DB thử nghiệm riêng.

## Bằng chứng kiểm tra ngày 2026-10-27

- Harness `tools/Step11Baseline.java`: javac/java Temurin 25.0.4.101, controller/
  service/Feign contract thật, 31 case đạt. Log/raw fixture tại
  `target/step-11-logs/java-baseline.log` và `baseline-java.json`. Rust so sánh
  30 case controller; missing-session được kiểm tra với filter thật (401).
- `cargo test --workspace --locked`: 16 pass; 11 test DB/JNI/export ignored
  trong lượt workspace không được tính đạt. Log `target/step-11-logs/workspace-tests.log`.
- `cargo test -p viewer-app --locked postgres_migrations_transaction_readiness_and_close_wado -- --ignored`:
  2/2 pass, không skip. Dùng DB từ TOML/profile hiện tại. Bao phủ cache/DB fallback,
  shared/expired/missing session, outside study, upstream status/header/body,
  thumbnail query, WADO-URI default/validation, media fallback, 16 MiB stream,
  client đọc chậm, drop body, truncated body, timeout trước/sau headers và connection
  refused. Log `target/step-11-logs/postgres-wado.log`.
- DB fallback so sánh record sau khi insert để dùng đúng độ chính xác timestamp
  PostgreSQL, sau đó kiểm chứng record không đổi. Một SESSION fixture có UUID riêng
  còn lại từ lượt assert thất bại ban đầu đã được nhận diện bằng cả UUID prefix,
  storeAuth giả và URL localhost rồi xóa; lượt cuối không để lại SESSION test.
- `cargo build --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo fmt --all -- --check`, `git diff --check`: đạt. Log lint/format tại
  `target/step-11-logs/`; Cargo.lock cập nhật dependency cho streaming reqwest.
- `python tools/smoke_wado.py`: đạt với binary debug thật, DB cấu hình và Postman
  CLI 1.75.0 cục bộ. Member `store` nhận WADO-RS/WADO-URI và upstream 401/403/404/
  429/500/503; kiểm chứng byte/header, scope500, missing401, cleanup session và
  graceful shutdown. Log `simulation.ndjson`, `simulation.err`, `app.log` tại
  `target/step-11-logs/`. Không publish mock hoặc gửi request tới PACS thật.

Giới hạn: Java baseline không phải full Spring container với PACS thật; fixture
đều là dữ liệu giả. Chưa kiểm chứng frontend xem pixel, PACS thật, release build
hoặc CI Windows/Ubuntu trong lượt này. Key image cục bộ, DICOM normalization/JNI
và prefetch được loại khỏi phạm vi theo quyết định người dùng, không tính là đã làm.
