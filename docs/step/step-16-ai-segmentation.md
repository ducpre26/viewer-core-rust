# Step 16 - UC-011 Phân đoạn ảnh bằng AI

## Tính năng

Chuyển workflow AI phân đoạn và tải kết quả cho 3D sang Rust.

**JNI:** Không, trừ DICOM validation có evidence cần dcm4che.

## Nguồn Java

- [module/ai/rest/SegmentationAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/ai/rest/SegmentationAPI.java)
- [module/ai/service/AISegmentationServiceImpl.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/ai/service/AISegmentationServiceImpl.java)
- [UC-011](../../../1pacs-viewer-core/docs/use_cases/UC-011-phan-doan-anh-bang-ai.md)

## Việc cần làm

- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: MODELS, SESSION và filesystem kết quả.
- [ ] Create/status/statistics/result workflow.
- [ ] Lấy DICOM store information từ session.
- [ ] Validate Study/Series Instance UID.
- [ ] Download result bằng atomic write; không ghi đè result đã tồn tại.
- [ ] Trả path/location phù hợp cho 3D pipeline.

## Kiểm tra hoàn thành

- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-011 main flow, A1..A3, BR-001, BR-002; concurrent duplicate download, partial result và path traversal.
- [ ] Đạt khi: result state/path/atomicity khớp postcondition và 3D consumer đọc được.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
