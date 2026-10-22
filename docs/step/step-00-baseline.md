# Step 0 - Nguồn tham khảo và quy tắc đối chiếu theo tính năng

## Tính năng

Giữ đầu vào Java đã thu thập để dùng khi cần. Step này không chặn init Rust; không yêu cầu export toàn bộ OpenAPI, fixture hoặc benchmark trước Step 1.

**JNI:** Không.

## Nguồn Java

- [Baseline đã thu thập](../baseline/README.md) — nguồn tham khảo, không phải điều kiện để init.
- [Mã nguồn Java](../../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/)

## Việc cần làm

- [ ] Giữ snapshot/inventory đã có; không xóa hoặc tự tái tạo làm mất bằng chứng cũ.
- [ ] Với mỗi step tính năng, ghi revision Java và chỉ đối chiếu route/service/entity/job/test thuộc tính năng đó; loại seal.
- [ ] Tái sử dụng baseline hiện có nếu phù hợp revision/phạm vi; fixture, schema và benchmark bổ sung tại step cần chúng.

## Kiểm tra hoàn thành

- [ ] Có nguồn tham khảo và quy tắc đối chiếu theo tính năng; không lấy baseline toàn hệ thống làm gate đầu vào Step 1.
- [ ] Rà soát endpoint/job toàn hệ thống chuyển sang Step 18; công việc đã làm trước đây không được coi là coverage hoàn chỉnh.

## Tiến độ

- Trạng thái: `in_progress`
- Đã làm: Checkout nhánh Java `codex/java-baseline-step-00` tại `c983e4eac389fa6b31f970cfee8f9fce5e20af4c`. Lưu 17 snapshot đầu vào và SHA-256; tạo inventory tĩnh 75 route (73 trong phạm vi, 2 seal loại trừ), mapping sơ bộ UC/service/annotation quyền; tạo checklist từ 14 UC và danh sách tên cấu hình. Xem [baseline evidence](../baseline/README.md).
- Tiếp theo: Có thể bắt đầu Step 1 ngay; bổ sung baseline đúng phạm vi khi thực hiện từng tính năng, rà soát toàn bộ ở Step 18.
- Vướng mắc: Runtime fixture/benchmark chưa có, nhưng không chặn init. P1–P4 còn mở; chỉ xử lý trước bài kiểm tra cần đến chúng.
- Kiểm tra gần nhất: Compile Java thành công trên Temurin 25.0.4.1+1 bằng `mvn -Dmaven.repo.local=C:/Users/Admin/.m2/repository -DskipTests compile`; chưa chạy test. `python tools/baseline_inventory.py` tạo 75 route duy nhất, đủ UC-001..UC-014, 2 seal loại trừ và 17 snapshot. Inventory chưa phải export Spring, entity trace chưa hoàn tất; giữ toàn bộ checkbox completion chưa đánh dấu.
- Cập nhật: 2026-10-22 — đổi sang init trước, đối chiếu theo tính năng; chưa thực hiện implementation.
