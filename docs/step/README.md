# Các step chuyển Java → Rust

[Plan chính](../SPRING_TO_RUST_JVM_JNI_MIGRATION_PLAN.md) mô tả mapping tính năng. Các file dưới đây giữ nguyên 20 step, chỉ rút gọn cách trình bày. Nguồn Java là điểm bắt đầu đối chiếu, chưa phải inventory đầy đủ mọi endpoint.

## Thứ tự thực hiện

Bắt đầu Step 1 ngay, không chờ Step 0. Step 0 giữ nguồn tham khảo đã thu thập. Step 1–2 chỉ init và kết nối DB; Step 3 chỉ JVM/facade smoke test. Từ Step 3a, đối chiếu Java trong phạm vi tính năng đang triển khai và dependency thực sự cần; không khảo sát toàn backend trước. Step 18 mới rà soát toàn bộ để phát hiện thiếu sót. Giữ thứ tự nghiệp vụ mặc định; Step 8–9 tái sử dụng auth/session từ Step 3a.

| Step | Công việc |
|---|---|
| 0 | [Nguồn tham khảo và quy tắc đối chiếu theo tính năng](step-00-baseline.md) |
| 1 | [Khởi tạo Rust application](step-01-rust-application.md) |
| 2 | [Kết nối PostgreSQL và nền tảng persistence](step-02-persistence.md) |
| 3 | [Khởi tạo JVM/JNI và facade tối thiểu](step-03-jvm-jni-foundation.md) |
| 3a | [Auth/session foundation](step-03a-auth-session-foundation.md) |
| 4 | [UC-009 Quản trị client và quyền](step-04-client-permission.md) |
| 5 | [UC-008 Quản lý cấu hình Viewer](step-05-viewer-config.md) |
| 6 | [UC-012 Quản lý model AI](step-06-ai-model-registry.md) |
| 7 | [UC-014 Theo dõi license và người dùng đồng thời](step-07-license-ccu.md) |
| 8 | [UC-001 Khởi tạo phiên xem ảnh 2D](step-08-create-session.md) |
| 9 | [UC-002 Quản lý và chia sẻ phiên 2D](step-09-session-lifecycle.md) |
| 10 | [UC-003 Tra cứu study DICOM](step-10-qido-study.md) |
| 11 | [UC-004 Truy xuất dữ liệu ảnh DICOM](step-11-wado-retrieval.md) |
| 12 | [UC-005 Tải nội dung DICOM](step-12-dicom-download.md) |
| 13 | [UC-006 Tiếp nhận và xuất key image](step-13-key-image.md) |
| 14 | [UC-007 Quản lý phiên dựng hình 3D](step-14-session-3d.md) |
| 15 | [UC-010 Phân tích bất thường bằng AI](step-15-ai-abnormal-detection.md) |
| 16 | [UC-011 Phân đoạn ảnh bằng AI](step-16-ai-segmentation.md) |
| 17 | [UC-013 In ảnh qua máy in DICOM](step-17-dicom-print.md) |
| 18 | [Hardening, full regression và hoàn tất phạm vi migration](step-18-hardening.md) |

## Quy tắc chung

- Không sử dụng skill ai-first-sdlc để tạo, sửa, review hoặc điều phối bộ tài liệu này; không áp dụng agent/capability/approval gate của skill.

- Rust dùng PostgreSQL riêng, phát hành tính năng đã hoàn tất cùng dependency, độc lập với Spring BE. **Không port bất kỳ logic seal nào.**
- Trong từng step tính năng, ghi revision, baseline/fixture và chỉ đọc controller cùng service/repository/job/test liên quan; đối chiếu [use case](../../../1pacs-viewer-core/docs/use_cases/) và [entity model](../../../1pacs-viewer-core/docs/entity_model.md). Nếu khác code, ghi rõ và xác minh trước phần việc bị ảnh hưởng.
- Giữ API, permission, error, dữ liệu và tác động nghiệp vụ theo fixture Java. Không tự đổi schema, password format hoặc business rule khi port. Fixture/log không chứa PHI hay credential.
- Checklist kiểm thử bao gồm main/alternative flow, postcondition và business rule của UC; kiểm chứng DB/file/metric khi liên quan. Không bỏ test bắt buộc vì file đã rút gọn.
- Chi tiết công nghệ, security và kiểm thử nằm trong [tham chiếu kỹ thuật](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md): mục 3, 5–7 cho JNI; 9–13 cho test, hiệu năng, logging và build. Số mục xuất hiện trong các step đều chỉ tài liệu này.
- Với JNI: clear exception trước đọc Throwable; timeout không giải phóng buffer/file/worker slot khi Java còn chạy; print kết quả chưa xác định không tự retry.

Entity/schema/repository được tạo tại step đầu tiên cần chúng, không mapping cả tám entity ở Step 2. Dependency dùng chung còn thiếu chỉ triển khai phần tối thiểu cần thiết, rồi hồi quy khi mở rộng. HTTP contract/error nghiệp vụ được chốt tại step tính năng; init chỉ có route hạ tầng.

## Cập nhật tiến độ

Trạng thái chỉ nằm trong từng file: `not_started` → `in_progress` → `in_review` → `done`; dùng `blocked` khi không tiếp tục được.

Sau mỗi phiên làm việc, cập nhật checkbox và mục **Tiến độ**: đã làm, việc tiếp theo, vướng mắc, kết quả kiểm tra kèm lệnh/file thực tế, ngày và người cập nhật. Nếu bị chặn, ghi điều kiện gỡ chặn. Giữ lại ghi chú quyết định/kết quả có giá trị khi tiếp tục phiên sau.

Chỉ `done` khi checklist và kiểm tra hoàn thành đạt, có evidence thực tế và phê duyệt nếu gate yêu cầu. Không tự ghi pass hoặc approval. Việc sửa tài liệu không làm tăng tiến độ implementation; xem trạng thái thực tế trong từng file step.

## Công nghệ và điểm còn mở

Dùng [T1–T13 đã chốt](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#decisions). [T7 bổ sung, T14, version và P1–P4](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#open-decisions) còn mở; ghi quyết định thực tế vào tiến độ khi có. Không tự điền tham số hoặc đổi kiến trúc.

[Nguồn tham khảo chính thức](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#technology-sources) dùng để đối chiếu công nghệ; phiên bản tài liệu phải khớp dependency khi khóa version. Step 18 kiểm tra toàn bộ, không cản phát hành sớm tính năng đã đạt yêu cầu.
