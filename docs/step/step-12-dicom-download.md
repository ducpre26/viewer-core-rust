# Step 12 - UC-005 Tải nội dung DICOM

## Tính năng

Chuyển download DICOM/JPEG/MP4 và ZIP sang Rust; giữ xử lý DICOM qua Java.

**JNI:** Render/transcode.

## Nguồn Java

- [module/file/rest/DownloadAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/file/rest/DownloadAPI.java)
- [module/file/service/DownloadServiceImpl.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/file/service/DownloadServiceImpl.java)
- [UC-005](../../../1pacs-viewer-core/docs/use_cases/UC-005-tai-noi-dung-dicom.md)

## Việc cần làm

- [ ] Bổ sung facade method parse/render/transcode cho download; tái sử dụng adapter Step 3. Trước call dữ liệu thực, hoàn thiện bounded queue, deadline, exception/reference handling và [ownership byte[]/file](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#jni-ownership); không đợi Step 18 mới kiểm tra an toàn.
- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: SESSION và filesystem download; không thêm entity DB nếu baseline không cần.
- [ ] Download study/series/instance với loại DICOM/JPEG/MP4 đúng phạm vi.
- [ ] Reject binary-default type theo behavior hiện tại.
- [ ] ZIP structure/naming và response streaming.
- [ ] DICOM transcode/render qua Java facade.
- [ ] Chốt T14 trước phần video export: giữ Java FFmpeg binding hay process do Rust quản lý; chưa tự chọn.
- [ ] Áp dụng [mục 5.7](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#jni-ownership) cho cancellation của render/transcode: giữ buffer/file lease/worker slot đến completion; TTL cleanup bỏ qua file đang dùng.

## Kiểm tra hoàn thành

- [ ] Test operation JNI/codec liên quan bằng corpus của tính năng; queue quá tải/hết hạn, cancellation, buffer/file lifetime và reference leak; không yêu cầu operation của tính năng chưa làm.
- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-005 main flow, A1..A3, BR-001..BR-003; golden ZIP, decoded JPEG, MP4, disk-full, cancellation và memory peak.
- [ ] Cancellation/disconnect khi transcode còn chạy: lease và backing memory còn hợp lệ; cleanup chỉ sau completion.
- [ ] Đạt khi: output mở được bằng tool độc lập, codec/transfer syntax và resource limits pass.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
