# Step 13 - UC-006 Tiếp nhận và xuất key image

## Tính năng

Chuyển nhận key image, lưu batch, retry và gửi STOW sang Rust.

**JNI:** Tạo/validate/ghi DICOM.

## Nguồn Java

- [module/stowrs/rest/StowRSAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/stowrs/rest/StowRSAPI.java)
- [module/stowrs/service/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/stowrs/service/)
- [module/keyimage/](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/keyimage/)
- [UC-006](../../../1pacs-viewer-core/docs/use_cases/UC-006-tiep-nhan-va-xuat-key-image.md)

## Việc cần làm

- [ ] Bổ sung facade method create-key-image/validate/ghi DICOM; tái sử dụng adapter Step 3. Trước call dữ liệu thực, hoàn thiện bounded queue, deadline, exception/reference handling và [ownership byte[]/file](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#jni-ownership); không đợi Step 18 mới kiểm tra an toàn.
- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: STUDY_KEY_IMAGE, SESSION, CONFIG và file key image.
- [ ] Parse multipart và validate batch/required attributes.
- [ ] Java facade tạo DICOM object/key image và bổ sung thuộc tính suy ra.
- [ ] Ghi pending batch, retry count và error description.
- [ ] Scheduler lấy tối đa 100 batch, retry `<= 3`, gửi STOW khi feature bật.
- [ ] Cleanup record/temp data sau thành công hoặc ingest failure.
- [ ] Áp dụng [mục 5.7](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#jni-ownership) cho JNI create/validate DICOM; cleanup không xóa file Java còn sử dụng.

## Kiểm tra hoàn thành

- [ ] Test operation JNI/codec liên quan bằng corpus của tính năng; queue quá tải/hết hạn, cancellation, buffer/file lifetime và reference leak; không yêu cầu operation của tính năng chưa làm.
- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-006 main flow, A1..A3, BR-001..BR-004; retry boundary, batch limit, disabled feature, partial failure và cleanup.
- [ ] Cancellation khi facade đang dùng file: không cleanup sớm; record/temp state vẫn được kiểm chứng theo contract.
- [ ] Đạt khi: DICOM output hợp lệ và pending/retry/cleanup state khớp postcondition.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
