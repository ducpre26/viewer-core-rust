# Step 18 - Hardening, full regression và hoàn tất phạm vi migration

## Tính năng

Kiểm tra toàn bộ tính năng Rust đã chuyển, đóng gói và xác nhận phát hành độc lập.

**JNI:** Kiểm thử adapter và packaged runtime.

## Nguồn Java

- [src/test/](../../../1pacs-viewer-core/src/test/)
- [docs/use_cases/](../../../1pacs-viewer-core/docs/use_cases/)

## Việc cần làm

- [ ] Rà soát toàn bộ endpoint/job/WebSocket/config trong Java (trừ seal), đối chiếu với các tính năng Rust đã làm; tái sử dụng inventory Step 0 nếu đúng revision.
- [ ] Tổng hợp baseline/fixture/migration từ từng step; ghi tính năng thiếu, dependency hoặc khác biệt còn mở và xử lý trước khi xác nhận toàn bộ migration hoàn tất.
- [ ] Chạy traceability matrix: mọi main/alternative/business rule có test evidence.
- [ ] Full OpenAPI/contract/database/DICOM/JNI/security/performance regression.
- [ ] Staging test với packaged Rust + JRE + facade/dependency JAR + native codec.
- [ ] Soak test cho session, WADO, JNI, 3D, license và print.
- [ ] Backup/restore rehearsal trên database riêng của Rust, deployment runbook và rollback drill cho release Rust.
- [ ] Xác minh toàn bộ tính năng đã triển khai lần lượt trong Rust; phát hành bản đầy đủ không yêu cầu dừng Spring BE.
- [ ] Kiểm chứng độc lập với Spring BE: database/config/storage Rust riêng; chỉ bật tính năng đã đạt gate và dependency, không yêu cầu dừng Spring.

## Kiểm tra hoàn thành

- [ ] Không còn endpoint/job trong phạm vi bị bỏ sót; các phần seal được ghi rõ loại trừ. Inventory và bằng chứng runtime được đối chiếu ở đây, không phải điều kiện init.
- [ ] Chạy đầy đủ các suite tại [mục 9.1–9.5](../SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#facade-tests); so sánh metric với ngưỡng được duyệt, lưu evidence staging/soak/security/SBOM và rollback của Rust.
- [ ] Đạt khi: UC-001..UC-014 đều pass; không có rule/entity discrepancy mở; Tech Lead, QA và Operations chấp nhận hoàn tất migration. Gate này không cản phát hành sớm các tính năng đã đạt gate cùng dependency của chúng.

## Tiến độ

- Trạng thái: `not_started`
- Đã làm: Chưa triển khai.
- Tiếp theo: Kiểm tra đầu vào theo [README](README.md), rồi bắt đầu checklist.
- Vướng mắc: Chưa đánh giá.
- Kiểm tra gần nhất: Chưa chạy.
- Cập nhật: 2026-10-22 — rút gọn tài liệu, không đổi tiến độ.
