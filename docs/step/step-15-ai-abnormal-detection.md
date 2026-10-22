# Step 15 - UC-010 Phân tích bất thường bằng AI

## Tính năng

Chuyển gọi AI phát hiện bất thường và xử lý kết quả sang Rust.

**JNI:** Không.

## Nguồn Java

- [module/ai/rest/InferenceAPI.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/ai/rest/InferenceAPI.java)
- [module/ai/service/AIAbnormalDetectionServiceImpl.java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/ai/service/AIAbnormalDetectionServiceImpl.java)
- [UC-010](../../../1pacs-viewer-core/docs/use_cases/UC-010-phan-tich-bat-thuong-bang-ai.md)

## Việc cần làm

- [ ] Đối chiếu đúng tính năng này trong Java: route/service/job/test, request/response, permission, lỗi; ghi revision và fixture/expected result trước khi port. Không yêu cầu khảo sát module ngoài dependency cần thiết.
- [ ] Mapping dữ liệu và migration/repository SeaORM trong phạm vi: MODELS đã có.
- [ ] Validate/select model by name.
- [ ] Build authenticated request tới AI service.
- [ ] Normalize study/series result.
- [ ] Map missing model và empty AI result.

## Kiểm tra hoàn thành

- [ ] API/output và tác động DB/file khớp baseline của tính năng này; kiểm tra constraint/transaction nếu có migration, hồi quy các chức năng đã dùng chung entity.
- [ ] UC-010 main flow, A1, A2 và BR-001; AI timeout/5xx/malformed response.
- [ ] Đạt khi: inference response/error contract pass và không ghi dữ liệu bền vững ngoài ý muốn.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Đọc nguồn Java của tính năng này và xác định dependency dùng chung cần có; lấy baseline trong phạm vi rồi triển khai.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
