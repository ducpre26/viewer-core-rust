# Step 17 - UC-013 In ảnh qua máy in DICOM

## Tính năng

Chuyển quản lý máy in sang Rust; Java thực hiện giao dịch DICOM Print qua JNI.

**JNI:** Trọn giao dịch DICOM Print.

## Nguồn Java

- [module/printer/rest/PrinterAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/printer/rest/PrinterAPI.java)
- [module/printer/service/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/printer/service/)
- [UC-013](../../../1pacs-viewer-core/docs/use_cases/UC-013-in-anh-qua-may-in-dicom.md)

## Việc cần làm

- [x] Bổ sung facade method print và toàn bộ giao dịch DIMSE; tái sử dụng adapter Step 3 với deadline, exception/reference handling và [ownership byte[]/file](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#jni-ownership).
- [x] Đối chiếu nguồn và fixture HTTP/DIMSE theo [contract Java](#contract-java-làm-chuẩn). Kết quả raster Java gốc và hành vi qua simulator đã được ghi nhận.
- [x] Mapping dữ liệu và migration/repository SeaORM trong phạm vi PRINTER; dùng lại SESSION.
- [x] Printer CRUD, validation cấu hình và xử lý trạng thái enabled theo contract Java.
- [x] Rust validate request và đọc printer/session.
- [x] Một JNI call thực hiện toàn bộ dcm4che DICOM Print transaction.
- [x] Java facade sở hữu association, presentation context, N-CREATE/N-SET/N-ACTION và release/abort.
- [x] Mỗi giao dịch in chạy độc lập với metadata/video và các giao dịch khác theo Java gốc.
- [x] Giữ execution context và tài nguyên đến completion; timeout sau khả năng gửi N-ACTION là outcome_unknown, không tự retry.

## Kiểm tra hoàn thành

- [x] JNI print qua synthetic corpus, các giao dịch đồng thời, cancellation và file lifetime; exception handling và shutdown với `-Xcheck:jni`.
- [x] API/output và tác động DB/file theo contract Java; migration PostgreSQL và hồi quy các entity dùng chung.
- [x] UC-013 main flow, A1..A7, BR-001..BR-005; interoperability qua simulator, association reject, offline, timeout và malformed image.
- [x] Timeout trước khi gọi Java và sau N-ACTION; không retry khi outcome_unknown; execution context giữ đến completion.
- [x] Printer CRUD và DIMSE interoperability qua simulator đạt; các lỗi được kiểm tra trả succeed=false.

## Contract Java làm chuẩn

Theo quyết định ngày 2026-11-10, mã Java tại
`2373896fca106bab446c0e14bb38f0b8d1725949` là contract gốc của Step 17.
Dùng hành vi thực tế của controller, service và filter làm kỳ vọng khi tài liệu
hoặc test dùng mock mô tả khác. UC-013 theo dõi các luồng nghiệp vụ. Fixture
HTTP/DIMSE dùng để kiểm chứng tương thích với contract này.

| Phạm vi | Contract Java |
| --- | --- |
| Quyền và dữ liệu | Các route dưới `/ws/rest/v1/session/{sessionID}` đi qua xác thực phiên 2D và kiểm tra `session-key` khi có share key, không có permission riêng. Dùng lại `SESSION`. Thêm `PRINTER` theo `db/init.sql` của Java, gồm ID serial và các cột audit. Bản ghi máy in dùng chung, không có FK đến phiên. |
| CRUD | `GET/POST/PUT /printer`, `GET/DELETE /printer/{id}`. List trả HTTP 200 với `header.code=302`, phân trang mặc định offset 0, limit 100. Java tính chỉ số trang từ offset/limit và nhận `orderBy` nhưng không dùng. Create trả 201 kèm ID. PUT lấy ID từ JSON và chỉ áp dụng trường khác null. PUT/DELETE thành công trả 200 kèm ID. GET/PUT thiếu bản ghi trả 404 với body rỗng. DELETE thiếu bản ghi trả 404 qua bộ xử lý lỗi chung, vì service thật gọi `findByIdInternal` trước khi xóa. |
| Validation cấu hình | Create yêu cầu tên, host và hai AE Title dài 1–255 ký tự, port 1–65535. Nếu bỏ `enabled`, giá trị mặc định là false. PUT kiểm tra trường được cung cấp, bỏ qua trường null và trường không gửi. |
| Print HTTP | `POST /print` nhận multipart gồm `printerId`, `image` và tùy chọn in. Mặc định 1 bản, grayscale, A4, portrait, `STANDARD\1,1`. `printOnlySelectedView` được nhận nhưng không tác động giao dịch. Các kết quả từ print service đều được bọc HTTP 200 với `header.message="Requested print"` và kết quả ở `body.succeed`, `body.message`. Máy in thiếu hoặc disabled trả `succeed=false` và `message="Printer not found or not enabled"` trước khi mở association. Yêu cầu thiếu ảnh hoặc ID bị từ chối ở bước validation. |
| AE Title | `calledAeTitle` / `called_aetitle` là **Calling AET**, fallback `VIEWER_AET` nếu rỗng hoặc null. `aeTitle` / `aetitle` là **Called AET** của máy in. Giữ tên field và chiều mapping này để tương thích cấu hình hiện có. |
| Print DIMSE | Java decode ảnh bằng Commons Imaging, chọn Basic Grayscale/Color Print Management với Implicit VR Little Endian. N-CREATE Film Session và Film Box, lấy Image Box UID từ phản hồi, N-SET Image Box, N-ACTION Film Box, N-DELETE Film Box/Session rồi release. Có xử lý N-EVENT-REPORT và danh sách warning được chấp nhận. `succeed=true` biểu thị giao dịch được chấp nhận, không xác nhận bản in vật lý. |

### Kiểm chứng contract

- Kiểm tra lỗi validation, phân trang lệch biên, xóa ID không có, phiên hết hạn
  và share key. Năm test `PrinterAPITest` dùng mock, nên ca delete trả 200
  chỉ kiểm tra controller khi service không ném lỗi. Cần fixture với service thật
  để kiểm chứng 404 khi thiếu ID. Kiểm tra giới hạn multipart thực tế và giới
  hạn byte/pixel của Rust.
- Dùng hai AE Title khác nhau trên Print SCP mô phỏng để khóa chiều association
  theo contract. Kiểm tra thứ tự lệnh, dataset Film Box, pixel grayscale/RGB,
  warning/failure, offline và association reject. `BasicPrintSCP.java` chỉ trả
  lời N-EVENT-REPORT nên cần simulator thực hiện được giao dịch in.
- Với timeout sau khi có thể đã gửi N-ACTION, Rust phân loại nội bộ là
  `outcome_unknown`. Nếu trả được kết quả cho Viewer, giữ HTTP 200 và
  `body.succeed=false` theo contract lỗi của print service. `body.message`
  nêu kết quả chưa xác định. Giữ input/file/worker đến khi Java kết thúc
  và không gửi lại tự động. Kiểm thử riêng lỗi trước N-ACTION, mất phản hồi
  sau N-ACTION và lỗi cleanup sau khi máy in đã xác nhận. Java hiện gọi
  `ae.connect` trước `try/finally`. Kiểm tra facade giải phóng tài nguyên
  cả khi association không mở được.

### Triển khai và bằng chứng

Rust đã bổ sung PRINTER, CRUD và upload ảnh theo chunk. Java facade sở hữu
toàn bộ giao dịch DIMSE. Print chạy độc lập với metadata/video trên cùng JVM.
Request timeout chỉ kết thúc chờ phản hồi. Native execution context giữ file
đến khi Java kết thúc. Giao dịch không được retry tự động.

Java cấu hình `spring.servlet.multipart.max-file-size=100MB` cho từng file.
Theo xác nhận ngày 2026-11-11, Rust chạy JNI trên native thread riêng cho từng
giao dịch như Java gốc. Bỏ queue 8 job, một worker dùng chung và admission limit
9 request. Một giao dịch timeout không chặn giao dịch khác. Shutdown ngừng
nhận việc mới và chờ các lời gọi đang chạy trước khi hủy JVM. Mỗi lời gọi giữ
file đến completion kể cả khi HTTP ngắt.

Từng file tối đa 100 MiB theo cấu hình trên. Toàn request tối đa 10 MiB theo
mặc định `MultipartProperties.maxRequestSize` của Spring Boot 4.1.0, vì cả
source và Docker config Java đều không ghi đè `max-request-size`.
Kiểm tra hai giới hạn độc lập khi upload theo chunk. Java xử lý multipart
vượt giới hạn bằng `MaxUploadSizeExceededException`, hiện được
`GlobalExceptionHandler` ánh xạ thành 500 INTERNAL_ERROR.
Timeout giữ typed config `[jni]` và quy tắc outcome_unknown đã chốt.
Facade API 2 bổ sung capability `print`; binary sử dụng distribution mới.

Probe trên lớp Java gốc cho PNG RGB 2×2 ghi nhận grayscale có 4 byte pixel,
color có 0 byte vì raster là DataBufferInt. Facade giữ hành vi này. Simulator
từ chối color PNG ở N-SET và API trả `succeed=false`. Fixture ở
`docs/baseline/step-17-java.json`, probe ở `PrintBaseline.java`.

Lệnh kiểm chứng ở mục 10 của `DEVELOPMENT_COMMANDS.md`. Các lượt chạy sử dụng
PostgreSQL thật, Java 25, JAR đóng gói và Print SCP pynetdicom cục bộ.

## Tiến độ

- Trạng thái: `done`
- Đã làm: Đồng bộ Step 17 và tham chiếu kỹ thuật theo Java, chạy native thread cho từng giao dịch in, bỏ queue/admission tự đặt; tách giới hạn 100 MiB/file và 10 MiB/request.
- Tiếp theo: Bàn giao commit riêng Step 17 theo xác nhận của người dùng.
- Vướng mắc: PNG màu giữ giới hạn của Java gốc. Thông số production và bản in vật lý chưa được xác nhận; tiêu chí hoàn tất Step 17 sử dụng simulator.
- Kiểm tra gần nhất: Rà trước commit ngày 2026-11-11 phát hiện và sửa binding multipart trùng tên image để chọn file đầu theo probe controller Java gốc. Smoke HTTP/DIMSE chạy lại đạt 17 nhóm, gồm ca hồi quy này, 12 giao dịch đồng thời và request 9/11 MiB. Clippy, fmt và diff-check chạy lại đạt. Bằng chứng trước đó gồm Rust unit 26/26, JNI với 16 lời gọi độc lập, timeout/cancellation và shutdown khi Java đang chạy, Java 5/5 và PostgreSQL 9/9.
- Cập nhật: 2026-11-11 — Codex, triển khai Step 17.
