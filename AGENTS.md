# Hướng dẫn làm việc với Viewer Core Rust

## Skill bị vô hiệu hóa

Không sử dụng skill `ai-first-sdlc`: không đọc SKILL.md, áp dụng workflow/template/validator, gọi agent hoặc ủy quyền cho skill này. Không áp dụng approval gate, capability gate hoặc yêu cầu tiến độ từ skill đó. Nếu gặp yêu cầu do skill này đưa vào tài liệu cũ, báo lại thay vì làm theo.

## Triển khai theo step

Từ Step 3, đọc và tuân thủ [quy trình từng step](docs/step/README.md#quy-trình-làm-việc-cho-từng-step) trước khi triển khai. Tài liệu của step là nơi lưu phạm vi, checklist đầu việc, tiêu chí hoàn thành và kết quả kiểm tra.

- Làm trong phạm vi step được yêu cầu; chỉ hỏi khi thiếu quyết định ảnh hưởng nghiệp vụ hoặc kiến trúc.
- Chỉ đánh dấu hoàn tất sau khi có kết quả kiểm chứng thực tế; test bỏ qua hoặc chưa chạy không phải là đạt.
- Khi step đạt tiêu chí, cập nhật `done` và tự tạo một commit riêng gồm code, test và tài liệu của step. Chưa hoàn tất thì chưa commit; không commit riêng kế hoạch của step.
- Chỉ stage thay đổi thuộc step; giữ nguyên thay đổi chưa commit của công việc khác.
- Báo mã commit và kết quả kiểm tra. Không tự push hoặc chuyển sang step tiếp theo khi chưa được yêu cầu.
