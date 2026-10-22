# Step 11 - UC-004 Truy xuất dữ liệu ảnh DICOM

## Tính năng

Chuyển proxy WADO-RS và kiểm tra phạm vi study sang Rust.

**JNI:** Chỉ parse/normalize khi baseline cần; streaming do Rust.

## Nguồn Java

- [module/wado/rest/WadoRsAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/wado/rest/WadoRsAPI.java)
- [module/wado/service/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/wado/service/)
- [UC-004](../../../1pacs-viewer-core/docs/use_cases/UC-004-truy-xuat-du-lieu-anh-dicom.md)

## Việc cần làm

- [ ] Bổ sung facade method parse/normalize chỉ nếu baseline yêu cầu; proxy thuần không cần operation JNI; tái sử dụng adapter Step 3. Trước call dữ liệu thực, hoàn thiện bounded queue, deadline, exception/reference handling và [ownership byte[]/file](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#jni-ownership); không đợi Step 18 mới kiểm tra an toàn.
- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: SESSION đã có; không tạo schema mới nếu không có yêu cầu baseline.
- [ ] Enforce study thuộc session.
- [ ] Proxy study/series/instance/frame/thumbnail/metadata/bulk data.
- [ ] Preserve status, content type, transfer syntax và safe response headers.
- [ ] Streaming/backpressure và client disconnect.
- [ ] DICOM JSON parse/normalize chỉ qua facade khi baseline cần.

## Kiểm tra hoàn thành

- [ ] Test operation JNI/codec liên quan bằng corpus của tính năng; queue quá tải/hết hạn, cancellation, buffer/file lifetime và reference leak; không yêu cầu operation của tính năng chưa làm.
- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-004 main flow, A1, A2, BR-001, BR-002; large stream, slow client, disconnect và upstream error.
- [ ] Đạt khi: WADO streaming/media contract pass mà không buffer toàn bộ payload ngoài yêu cầu.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
