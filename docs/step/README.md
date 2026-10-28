# Các step chuyển Java → Rust

[Kế hoạch chính](../SPRING_TO_RUST_JVM_JNI_MIGRATION_PLAN.md) chia việc chuyển Java sang Rust thành 20 step dưới đây. Nguồn Java trong từng step là điểm bắt đầu đối chiếu, chưa liệt kê đủ mọi endpoint.

## Thứ tự thực hiện

- Bắt đầu từ Step 1; Step 0 lưu nguồn tham khảo, không chặn triển khai.
- Step 1–2 chỉ khởi tạo ứng dụng và kết nối DB; Step 3 chỉ khởi tạo JVM/facade và chạy smoke test.
- Từ Step 3a, chỉ đối chiếu Java cho tính năng và dependency cần dùng. Step 18 rà soát toàn backend để tìm phần còn thiếu.
- Làm theo thứ tự dưới đây; Step 8–9 dùng lại auth/session từ Step 3a.

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

## Contract cấu hình chung

Rust không dùng Orthanc transcode/native, khác Java. Phạm vi và tiến độ download
ở [Step 12](step-12-dicom-download.md).

Theo [contract TOML](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#toml-config)
được người dùng xác nhận: giữ nguyên Summer nạp app.toml, overlay theo profile,
interpolation env và RUST_LOG override. Module nhận typed config từ registry;
không tự đọc env cho DB/auth/session/JNI. Không vá framework hoặc thêm --config.
Các bằng chứng cũ được giữ; kết quả refactor mới ghi tại mục 2.4 tham chiếu kỹ thuật.
Không đổi nghiệp vụ/schema hoặc snapshot Java vì thay nguồn cấu hình.

## Quy tắc chung

- Không dùng skill `ai-first-sdlc` để tạo, sửa, review hay điều phối tài liệu; không áp dụng agent hoặc gate của skill.
- Rust dùng PostgreSQL riêng. Tính năng hoàn tất cùng dependency được phát hành độc lập với Spring BE. **Không port logic seal.**
- Mỗi step tính năng ghi revision Java và baseline/fixture; chỉ đọc controller, service, repository, job và test liên quan. Đối chiếu [use case](../../../1pacs-viewer-core/docs/use_cases/) và [entity model](../../../1pacs-viewer-core/docs/entity_model.md); nếu khác code, ghi rõ và xác minh trước khi làm phần bị ảnh hưởng.
- Giữ API, quyền, lỗi, dữ liệu và tác động nghiệp vụ theo fixture Java. Không tự đổi schema, định dạng mật khẩu hay quy tắc nghiệp vụ. Fixture/log không chứa PHI hoặc thông tin xác thực.
- Kiểm thử đủ luồng chính, luồng thay thế, hậu điều kiện và quy tắc nghiệp vụ của UC; kiểm chứng DB, file, metric khi liên quan. Không bỏ test bắt buộc khi rút gọn tài liệu.
- Tạo entity/schema/repository khi step đầu tiên cần dùng, không tạo cả tám entity ở Step 2. Dependency dùng chung chỉ làm phần cần thiết; kiểm thử hồi quy khi mở rộng. Chốt HTTP contract và lỗi nghiệp vụ tại step tính năng; bước khởi tạo chỉ có route hạ tầng.

[Tham chiếu kỹ thuật](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md) quy định công nghệ, bảo mật và kiểm thử: mục 3, 5–7 về JNI; mục 9–13 về test, hiệu năng, logging và build. Các số mục trong tài liệu step đều trỏ đến tài liệu này.

Với JNI: clear exception trước khi đọc Throwable; không giải phóng buffer/file/worker slot sau timeout nếu Java còn chạy; không tự retry lệnh in khi chưa xác định kết quả.

## Quy trình làm việc cho từng step

Áp dụng từ Step 3:

1. **Đọc kế hoạch:** xác định phạm vi, kết quả cần đạt, dependency và tiêu chí hoàn thành; đọc phần Java liên quan.
2. **Triển khai:** làm theo đầu việc và checklist có sẵn, đúng thứ tự; không chia lại hoặc tạo checklist trùng. Tự xử lý chi tiết kỹ thuật; chỉ hỏi khi thiếu quyết định về nghiệp vụ, kiến trúc hoặc cần xác nhận commit.
3. **Kiểm tra và sửa lỗi:** chọn lệnh từ [DEVELOPMENT_COMMANDS.md](../DEVELOPMENT_COMMANDS.md), kiểm tra tiêu chí của nhiệm vụ, step và hồi quy phần bị ảnh hưởng. Lệnh hiện có chỉ kiểm tra nền tảng HTTP/persistence, chưa đủ cho mọi tính năng mới. Nếu cần lệnh mới, thống nhất với người quản lý về mục đích và kết quả cần kiểm chứng trước khi bổ sung.
4. **Rà soát và cập nhật:** đọc lại thay đổi, cập nhật checklist và mục **Tiến độ** theo quy định bên dưới. Chỉ đánh dấu hoàn thành khi có kết quả kiểm chứng; test chưa chạy hoặc bị bỏ qua không tính là đạt.
5. **Đề xuất commit:** khi step đạt tiêu chí, đánh dấu `done`, báo thay đổi, kết quả kiểm tra và nội dung commit dự kiến. Thực hiện theo quy tắc commit bên dưới.

## Quy tắc commit

- Chỉ commit khi step hoàn tất và người dùng đã xác nhận.
- Mỗi step một commit gồm code, test và tài liệu liên quan; không commit riêng kế hoạch khi step chưa xong.
- Dùng đúng mã step, kể cả `step-03a`. Ví dụ: `feat(step-03): implement JVM JNI foundation`.
- Chỉ stage thay đổi của step; giữ nguyên thay đổi khác chưa commit.
- Báo mã commit sau khi tạo. Không tự push hoặc chuyển step khi chưa được yêu cầu.

## Cập nhật tiến độ

Chỉ ghi trạng thái trong từng file step: `not_started` → `in_progress` → `in_review` → `done`. Chưa đạt tiêu chí thì giữ `in_progress` hoặc `in_review`; dùng `blocked` khi không thể tiếp tục và ghi điều kiện gỡ chặn.

Sau mỗi phiên, cập nhật checkbox và mục **Tiến độ**: việc đã làm, việc tiếp theo, vướng mắc, kết quả kiểm tra kèm lệnh/file thực tế, ngày và người cập nhật. Giữ các quyết định và kết quả cần cho phiên sau.

Chỉ `done` khi hoàn thành checklist và kiểm tra đạt, có bằng chứng thực tế. Sửa tài liệu không làm tăng tiến độ triển khai.

## Công nghệ và điểm còn mở

Dùng [T1–T14 đã chốt](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#decisions); T14 giữ Java FFmpeg binding theo xác nhận người dùng 2026-10-27. [T7 bổ sung, phiên bản và P1–P4](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#open-decisions) còn mở; ghi vào **Tiến độ** khi có quyết định. Không tự điền tham số hoặc đổi kiến trúc.

Đối chiếu công nghệ với [nguồn chính thức](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#technology-sources); tài liệu phải khớp phiên bản dependency đã chốt. Không cần chờ Step 18 để phát hành tính năng đã đạt yêu cầu.
