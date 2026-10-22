# Step 17 - UC-013 In ảnh qua máy in DICOM

## Tính năng

Chuyển quản lý máy in sang Rust; Java thực hiện giao dịch DICOM Print qua JNI.

**JNI:** Trọn giao dịch DICOM Print.

## Nguồn Java

- [module/printer/rest/PrinterAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/printer/rest/PrinterAPI.java)
- [module/printer/service/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/printer/service/)
- [UC-013](../../../1pacs-viewer-core/docs/use_cases/UC-013-in-anh-qua-may-in-dicom.md)

## Việc cần làm

- [ ] Bổ sung facade method print và toàn bộ giao dịch DIMSE; tái sử dụng adapter Step 3. Trước call dữ liệu thực, hoàn thiện bounded queue, deadline, exception/reference handling và [ownership byte[]/file](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#jni-ownership); không đợi Step 18 mới kiểm tra an toàn.
- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: PRINTER và SESSION.
- [ ] Printer CRUD và validation host/port/AE Title/enabled.
- [ ] Rust validate request và đọc printer/session.
- [ ] Một JNI call thực hiện toàn bộ dcm4che DICOM Print transaction.
- [ ] Java facade sở hữu association, presentation context, N-CREATE/N-SET/N-ACTION và release/abort.
- [ ] Worker pool/circuit breaker riêng cho print.
- [ ] Áp dụng [mục 5.7](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#jni-ownership): giữ worker slot/tài nguyên đến completion; timeout sau khả năng gửi N-ACTION là outcome_unknown, không tự retry.

## Kiểm tra hoàn thành

- [ ] Test operation JNI/codec liên quan bằng corpus của tính năng; queue quá tải/hết hạn, cancellation, buffer/file lifetime và reference leak; không yêu cầu operation của tính năng chưa làm.
- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-013 main flow, A1..A3, BR-001..BR-004; simulator/printer interoperability, association reject, offline, timeout và malformed image.
- [ ] Timeout trước/sau khả năng gửi N-ACTION: không báo success giả, không retry tự động khi outcome_unknown; worker slot chỉ được trả sau completion.
- [ ] Đạt khi: printer CRUD + real/simulated DIMSE interoperability pass; lỗi không bao giờ báo success giả.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
