# Step 6 - UC-012 Quản lý model AI

## Tính năng

Chuyển CRUD model AI, cấu hình kết nối và validation sang Rust.

**JNI:** Không.

## Nguồn Java

- [module/ai/rest/ModelController.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/ai/rest/ModelController.java)
- [module/ai/service/ModelServiceImpl.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/ai/service/ModelServiceImpl.java)
- [UC-012](../../../1pacs-viewer-core/docs/use_cases/UC-012-quan-ly-model-ai.md)

## Việc cần làm

- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: MODELS.
- [ ] List/get/create/update/delete model.
- [ ] Enforce `model_name` bắt buộc/duy nhất và `url` bắt buộc.
- [ ] Bảo toàn authentication/config JSON semantics.
- [ ] Chưa tự thêm authorization vì precondition hiện ghi không có cơ chế riêng.

## Kiểm tra hoàn thành

- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-012 main flow, A1, A2, BR-001, BR-002 và unique constraint race.
- [ ] Đạt khi: model registry tương thích contract và database effect.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Rà soát phạm vi, nguồn Java và nền tảng Rust sau commit Step 5 `f027f8e`. Chưa triển khai hoặc hoàn thành baseline runtime; giữ nguyên checklist.
- Tiếp theo: Xác minh ba điểm validation/JSON/audit bên dưới bằng Java + PostgreSQL trong môi trường test riêng, ghi fixture/expected result rồi triển khai theo checklist hiện có.
- Vướng mắc: Source/UC/schema chưa thống nhất đủ để chốt hành vi ghi model. Lần chạy test Java offline dừng trước khi chạy test do dependency Maven; chưa có kết quả kiểm chứng DB.
- Kiểm tra gần nhất: Đọc ModelController/Service/Repository, ba DTO, Model/AuditableSerialIDEntry, base CRUD/response/error handler, WebRequestFilter/WebSecurityConfig, UC-012, entity model và db/init.sql tại Java revision `c983e4eac389fa6b31f970cfee8f9fce5e20af4c`; đối chiếu router/migration/response/auth Rust. `mvn -o '-Dmaven.repo.local=C:/Users/Admin/.m2/repository' '-Dtest=ModelControllerTest,ModelServiceImplTest' test` thất bại ở resources plugin vì cache không resolve được `maven-filtering:3.5.0` và `plexus-utils:3.6.0` theo repository ID central. Không tính là test pass; report cũ ngày 2026-10-06 không phải kết quả phiên này.
- Cập nhật: 2026-10-26 — Codex; kiểm tra trước triển khai theo yêu cầu người dùng, chưa commit.

## Kết quả kiểm tra trước triển khai — 2026-10-26

Phạm vi là 5 thao tác tại `/ws/rest/v1/ai/model`: GET collection, GET `/{id}`,
POST collection, PUT collection (ID trong body), DELETE `/{id}`. Không có
job/JNI hoặc lời gọi máy chủ AI trong CRUD registry. Chưa có route, entity hay
migration MODELS trong Rust. Auth hiện có tương thích: route model không thuộc
nhánh `/session/`, `/client/`, `/admin/` của WebRequestFilter; controller không
có guard riêng. Cần kiểm thử HTTP không có credential khi port.

Ba điểm phải xác minh trước khi chốt baseline ghi dữ liệu:

1. **Validation khác nhau giữa create và update.** ModelCreateDto dùng
   `@NotNull` + `@Size(min=1)` cho modelName/url và không trim: null/rỗng bị từ
   chối nhưng chuỗi toàn khoảng trắng qua được DTO. ModelUpdateDto không có
   constraint; trường non-null được Java trim và có thể thành chuỗi rỗng,
   null/missing giữ nguyên. Vì vậy yêu cầu “bắt buộc” trong UC-012/đầu việc
   không đồng nghĩa với NotBlank trên cả POST/PUT. Không dùng nguyên helper
   `clean` của Step 5 vì helper đó biến chuỗi blank thành null. Nếu cần siết
   validation so với Java, phải xác nhận thay đổi nghiệp vụ trước.
2. **configJson là String ở API/entity nhưng schema là jsonb.** Model không có
   mapping JSON hoặc cast được khai báo; entity model cũng mô tả String.
   authentication là chuỗi tùy chọn, không có parser/validation cấu trúc trong
   CRUD; DTO update giữ nguyên trường null. Cần kiểm chứng round-trip JSON hợp
   lệ, JSON sai, chuỗi rỗng, null/missing và lỗi rollback trên PostgreSQL thật;
   chưa kết luận Java ghi thành công hoặc đổi API thành JSON object.
3. **Audit có nguy cơ làm POST thất bại với schema init.sql.** created_time và
   creator_id là NOT NULL có default trong DB, nhưng AuditableSerialIDEntry
   chỉ có field/@Column, không có callback/listener tự gán; ModelCreateDto và
   base create không gán hai trường này. Cần kiểm chứng INSERT thực tế có gửi
   null khiến default không được dùng hay không. Không tự thêm giá trị audit
   hoặc thay schema khi chưa xác định hành vi Java.

Các chi tiết contract cần đưa vào baseline/kiểm thử trong checklist hiện có:
GET/PUT model thiếu trả 404 body rỗng; DELETE thiếu đi qua base service và trả
lỗi NOT_FOUND. Unique model_name được DB enforce; không có nhánh xử lý trùng
riêng ở service nên không tự quy định 409. Cần test cả update trùng và create
đồng thời, kiểm tra rollback và dữ liệu cuối. List dùng pagination giống Step
4/5, message `Found N models`; response model có id/modelName/url/authentication/
configJson. Hai bộ test Java hiện dùng mock repository/service, chưa kiểm tra
PUT, unique race, JSON persistence hay audit với PostgreSQL thật.
