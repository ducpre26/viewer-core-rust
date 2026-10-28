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

- Trạng thái: `in_progress` — chỉ phần đóng gói thử nghiệm theo yêu cầu ngày 2026-10-28; chưa bắt đầu full regression/hardening.
- Đã làm: Lưu [plan Docker](../DOCKER_DEPLOYMENT_PLAN.md), Dockerfile nhiều stage, build scripts Windows/Linux, Compose và TOML mẫu riêng. Giữ user mặc định của image theo người dùng; chỉ mount config và data, Java/facade nằm trong image. Facade nhận Git metadata cho build không có .git.
- Tiếp theo: Build image Linux amd64, kiểm chứng chính runtime image với DB/JNI/ZIP/MP4, volume và shutdown theo [runbook](../../deploy/docker/README.md); tiếp tục các mục chưa làm trong checklist trên khi được yêu cầu.
- Vướng mắc: Máy hiện tại chưa có Docker CLI/engine; chưa có bằng chứng Docker build/runtime. Thông số mẫu chỉ cho test, P1–P4 production vẫn mở.
- Kiểm tra gần nhất: `python tools/test_facade_metadata.py` đạt 3 test (explicit metadata không gọi Git, fallback local, từ chối metadata thiếu/sai); Python AST, TOML và Compose YAML parse đạt. PowerShell parser và Git Bash `bash -n` đạt; cả hai build script đã kiểm tra bằng Docker giả lập cho build/export, metadata, build lỗi và engine Windows. LF và `git diff --check` đạt. Đọc manifest công khai Docker Hub xác nhận Temurin 25 JDK Noble có linux/amd64 và ghim digest. Chưa chạy `docker compose config`, build image hay container thật; mock/parse không thay bằng chứng runtime.
- Cập nhật: 2026-10-28 — đóng gói theo yêu cầu người dùng, chưa commit/push/deploy; các checkbox hoàn tất vẫn giữ mở.
