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

- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: SESSION đã có; không tạo schema mới nếu không có yêu cầu baseline.
- [ ] Validate session và Patient ID.
- [ ] Normalize offset/limit chính xác theo business rule.
- [ ] Forward QIDO query và store authentication.
- [ ] Preserve current behavior của sort parameter.
- [ ] Map empty/missing-session response đúng status baseline.

## Kiểm tra hoàn thành

- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-003 main flow, A1..A3, BR-001..BR-003; upstream timeout/auth/error.
- [ ] Đạt khi: QIDO contract và pagination normalization pass.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
