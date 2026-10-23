# Step 0 - Nguồn tham khảo và quy tắc đối chiếu theo tính năng

## Tính năng

Giữ đầu vào Java đã thu thập để dùng khi cần. Theo xác nhận của người dùng ngày 2026-10-23, Step 0 tiếp tục `in_progress` cho đến khi có baseline runtime đầy đủ. Điều kiện này không chặn init Rust hoặc việc thực hiện các step tính năng; OpenAPI, fixture và benchmark không phải đầu vào bắt buộc trước Step 1.

**JNI:** Không.

## Nguồn Java

- [Baseline đã thu thập](../baseline/README.md) — nguồn tham khảo, không phải điều kiện để init.
- [Mã nguồn Java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/)

## Việc cần làm

- [x] Giữ snapshot/inventory đã có; không xóa hoặc tự tái tạo làm mất bằng chứng cũ.
- [ ] Với mỗi step tính năng, ghi revision Java và chỉ đối chiếu route/service/entity/job/test thuộc tính năng đó; loại seal.
- [ ] Tái sử dụng baseline hiện có nếu phù hợp revision/phạm vi; fixture, schema và benchmark bổ sung tại step cần chúng.

## Kiểm tra hoàn thành

- [x] Có nguồn tham khảo và quy tắc đối chiếu theo tính năng; không lấy baseline toàn hệ thống làm gate đầu vào Step 1.
- [ ] Thu baseline runtime đầy đủ theo [danh sách còn thiếu](../baseline/README.md#remaining-work--per-feature-and-final-coverage-review): mappings/OpenAPI thực tế, auth và tác động DB, fixture tổng hợp không chứa PHI/credential, DICOM/print/error, benchmark với workload và môi trường ghi nhận.
- [ ] Rà soát endpoint/job toàn hệ thống chuyển sang Step 18; công việc đã làm trước đây không được coi là coverage hoàn chỉnh.

## Tiến độ

- Trạng thái: `in_progress`
- Đã làm: Checkout nhánh Java `codex/java-baseline-step-00` tại `c983e4eac389fa6b31f970cfee8f9fce5e20af4c`. Lưu 17 snapshot đầu vào và SHA-256; tạo inventory tĩnh 75 route (73 trong phạm vi, 2 seal loại trừ), mapping sơ bộ UC/service/annotation quyền; tạo checklist từ 14 UC và danh sách tên cấu hình. Xem [baseline evidence](../baseline/README.md).
- Tiếp theo: Thu và đối chiếu baseline runtime; cập nhật fixture theo từng step tính năng và rà soát toàn bộ ở Step 18. Không chuyển Step 0 sang `done` trước khi hoàn thành baseline runtime.
- Vướng mắc: Chưa có môi trường Java runtime cô lập với DB và dịch vụ DICOM giả lập; thiếu runtime OpenAPI, fixture và benchmark. P1–P4 còn mở cho các bài kiểm tra cần đến chúng. Những mục này không chặn Step 1.
- Kiểm tra gần nhất: Compile Java thành công trên Temurin 25.0.4.1+1 bằng `mvn -Dmaven.repo.local=C:/Users/Admin/.m2/repository -DskipTests compile`; chưa chạy test. `python tools/baseline_inventory.py` tạo 75 route duy nhất, đủ UC-001..UC-014, 2 seal loại trừ và 17 snapshot. Ngày 2026-10-23, kiểm tra đọc-only lại 17 SHA-256: không có mismatch; inventory có 75 cặp method/path duy nhất. Inventory chưa phải export Spring; entity trace chưa hoàn tất. Hai checkbox đã đánh dấu chỉ xác nhận bằng chứng tĩnh hiện có, không xác nhận baseline runtime.
- Cập nhật: 2026-10-23 — Codex ghi nhận quyết định của người dùng: giữ `in_progress` đến khi có baseline runtime đầy đủ.
