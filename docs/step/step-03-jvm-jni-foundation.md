# Step 3 - Khởi tạo JVM/JNI và facade tối thiểu

## Tính năng

Sau khung Rust/DB, tạo kết nối JVM 25 và facade không Spring. Chưa đối chiếu hoặc triển khai toàn bộ parse/render/transcode/key-image/print.

**JNI:** Có — JVM startup và facade smoke test.

## Nguồn Java

- [pom.xml Java](../../../1pacs-viewer-core/pom.xml): chỉ lấy dependency DICOM hiện tại.
- [Quyết định T7–T13](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#decisions).

## Việc cần làm

- [ ] Khóa jni-rs, tạo facade JAR + dependency directory; load đúng libjvm Java 25 và tạo một JVM/process.
- [ ] Facade có apiVersion/capabilitiesJson; chỉ công bố operation/codec đã thực sự triển khai và kiểm chứng.
- [ ] Smoke call qua worker JNI riêng, attach/detach đúng thread và local frame; không block Tokio.
- [ ] Xử lý exception theo [mục 5.5](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#jni-exceptions); dùng lỗi tổng hợp để kiểm tra.
- [ ] Khi JNI bật, readiness kiểm tra JVM/facade version; startup failure rõ ràng, shutdown có giới hạn; lỗi không phục hồi restart process.
- [ ] Chuyển worker scheduling/byte[]/file ownership và DICOM operation sang step đầu tiên sử dụng chúng; không chạy golden corpus/print suite trước khi có tính năng.

## Kiểm tra hoàn thành

- [ ] Smoke test JVM/facade đúng bản đóng gói; sai path/classpath/version bị từ chối.
- [ ] Kiểm tra exception, local reference và attach/detach/shutdown dưới -Xcheck:jni, chưa cần DICOM corpus hoặc printer.
- [ ] Chỉ đóng foundation; byte/file transport, concurrency, cancellation, codec và print được kiểm thử tại step sử dụng và hồi quy ở Step 18.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Thực hiện checklist trong phạm vi step này; không chờ baseline toàn bộ Java.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — đổi sang init trước, đối chiếu theo tính năng; chưa thực hiện implementation.
