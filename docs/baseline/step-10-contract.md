# Baseline Step 10 — UC-003

Đối chiếu 2026-10-27 trước khi port, Java revision `2373896fca106bab446c0e14bb38f0b8d1725949`.
Nguồn: StudyAPI, StudyServiceImpl, StudyFilter, StudyGetDto, QidoServiceImpl,
QidoProxy, DicomWebUtil, APIListResponseHeader và WebRequestFilter.

- POST `/ws/rest/v1/study/search`, JSON body `sessionID`, `patientID` bắt buộc;
  sessionID không rỗng, patientID được phép rỗng. Không thêm permission guard.
- Query offset/limit i32 mặc định 0/100. QIDO offset âm về 0, limit <= 0 về 20;
  response header giữ offset/limit đầu vào. orderBy được nhận nhưng bỏ qua.
  Chuỗi query rỗng dùng mặc định; sai kiểu/ngoài i32 trả 500 INTERNAL_ERROR
  (đã xác minh bằng MockMvc với GlobalExceptionHandler).
- Session lookup theo body, cache rồi DB như Step 8/9. Không tìm thấy: 204, body rỗng.
- GET `{storeUrl}/studies`; forward storeAuth qua Authorization nếu khác null;
  query PatientID, PatientName, AccessionNumber, ModalitiesInStudy,
  StudyInstanceUID, StudyDate (start-end; cho phép thiếu một đầu), offset, limit.
  Không thêm lọc theo study set của session.
- Kết quả 200: header code 302, message `Found N studies`, datetime,
  offset, limit, totalRecords=N; body là danh sách DTO, bỏ field null.
- DTO lấy giá trị đầu của tags 00100020/0020000D/00100010/00180015/
  00081030/00080020/00080030/00080061/00080050 tương ứng
  pid/studyInstanceUID/patientName/bodyPart/description/studyDate/studyTime/
  modalityType/accessionNumber. PN giữ các nhóm theo định dạng DICOM.
  Bỏ chuỗi rỗng, trim padding; thiếu VR suy ra từ tag. Root object không phải
  danh sách dataset nên trả [] theo JSONReader Java đã chạy.
- Upstream không có body, JSON rỗng/sai hoặc []: 200 danh sách rỗng.
  Upstream non-2xx, lỗi mạng/timeout: 500 INTERNAL_ERROR; không trả secret/PHI.
- Không ghi DB/file; dùng SESSION hiện có, không migration và không JNI.

Khác UC-003 A1: tài liệu ghi 204 cho không có kết quả; code Java hiện tại trả 200
với body []. Giữ code làm baseline; A2 vẫn 204. Header phân trang không phản ánh
giá trị normalize là hành vi Java cần bảo toàn. Các fixture chỉ dùng dữ liệu giả.

Các trường hợp cần kiểm chứng: default/negative/zero pagination, orderBy ignored,
mọi filter, date open range, patientID rỗng/null, session rỗng/null/missing,
PN/multivalue/null mapping, upstream []/204/malformed/401/403/500/disconnect/timeout.
