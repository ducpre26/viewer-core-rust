# Kế hoạch chuyển tính năng Java Spring sang Rust

**Trạng thái:** Draft. **Cách làm:** chuyển từng tính năng tương ứng 1–1, kiểm tra đạt rồi chuyển tính năng tiếp theo.

## 1. Mục tiêu và nguyên tắc

Mỗi tính năng đang có trong Java Spring BE sẽ có chức năng tương ứng trong Rust. Giữ hành vi mà frontend và các hệ thống tích hợp đang sử dụng: API, dữ liệu trả về, quyền truy cập, xử lý lỗi và kết quả nghiệp vụ. Mapping 1–1 ở mức tính năng, không yêu cầu sao chép từng class Java thành một file Rust.

- Rust chạy độc lập, dùng **PostgreSQL riêng**, không dùng chung database với Spring BE.
- Triển khai và phát hành lần lượt các tính năng đã hoàn tất cùng dependency; không cần đợi chuyển hết hoặc dừng Spring BE.
- Rust quản lý API, nghiệp vụ, auth/session, database, cache, scheduler, HTTP proxy và Python/Trame.
- Cấu hình vận hành thống nhất bằng [TOML](SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#toml-config) theo quyết định 2026-10-26; không đọc env làm nguồn cấu hình từng module. Contract đã chốt, refactor code/harness/CI chưa triển khai.
- **Giữ dcm4che trên JVM nhúng qua JNI** cho xử lý DICOM và in. JVM không chạy Spring hoặc mở HTTP server. Không dùng GraalVM Native Image.
- Init Rust và kết nối DB trước; baseline/fixture Java được thu theo từng tính năng khi triển khai, pipeline Rust kiểm thử độc lập.

## 2. Bảng ánh xạ tính năng Java → Rust

Cột Rust mô tả phần cần mang sang, chưa phải xác nhận đã triển khai. Theo dõi tiến độ tại file step tương ứng.

| Tính năng Java | Điểm vào Java hiện tại | Tính năng tương ứng cần có trong Rust | Giữ Java qua JNI | Step hiện tại |
|---|---|---|---|---|
| UC-001 — Tạo phiên xem ảnh 2D | [ClientAPI + session](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/client/ClientAPI.java) | Tạo session và URL Viewer; giữ Basic Auth, IP allow-list, expiry và permission snapshot. | Không | [Step 8](step/step-08-create-session.md) |
| UC-002 — Quản lý và chia sẻ phiên 2D | [SessionAPI + session](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/session/rest/SessionAPI.java) | Get/update/end/share session; cache trước, DB riêng sau; giữ share key và cleanup hết hạn. | Không | [Step 9](step/step-09-session-lifecycle.md) |
| UC-003 — Tra cứu study | [StudyAPI + study/qido](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/study/rest/StudyAPI.java) | Giữ tìm kiếm, filter, phân trang, sort và response; gọi PACS bằng QIDO-RS. | Không mặc định | [Step 10](step/step-10-qido-study.md) |
| UC-004 — Truy xuất ảnh DICOM | [WadoRsAPI + wado](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/wado/rest/WadoRsAPI.java) | Proxy study/series/instance/frame/thumbnail/metadata/bulk data; giữ study scope, header và streaming. | Chỉ khi cần normalize DICOM | [Step 11](step/step-11-wado-retrieval.md) |
| UC-005 — Tải DICOM/JPEG/MP4 | [DownloadAPI + file](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/file/rest/DownloadAPI.java) | Giữ phạm vi download, ZIP/naming và output; Rust điều phối file/FFmpeg, Java xử lý DICOM. | Render/transcode | [Step 12](step/step-12-dicom-download.md) |
| UC-006 — Nhận và xuất key image | [StowRSAPI + stowrs/keyimage](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/stowrs/rest/StowRSAPI.java) | Nhận multipart, tạo DICOM, lưu pending batch, retry và gửi STOW; giữ cleanup. | Tạo/validate/ghi DICOM | [Step 13](step/step-13-key-image.md) |
| UC-007 — Phiên dựng hình 3D | [Session3DAPI + session3d](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/session3d/rest/Session3DAPI.java) | Giữ create/list/get/delete/reset, process pool Python/Trame, port allocation và WebSocket. | Không cho orchestration | [Step 14](step/step-14-session-3d.md) |
| UC-008 — Cấu hình Viewer | [ConfigPublicAPI + phần config của AdminAPI](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/util/rest/ConfigPublicAPI.java) | Giữ CRUD config, validation, cache/invalidation và quyền sửa overlay. | Không | [Step 5](step/step-05-viewer-config.md) |
| UC-009 — Client và quyền | [Phần client/permission của AdminAPI](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/util/rest/AdminAPI.java) | Giữ CRUD client/permission, gán/bỏ quyền, admin-only và mapping DB. | Không | [Step 4](step/step-04-client-permission.md) |
| UC-010 — AI phát hiện bất thường | [InferenceAPI + ai](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/ai/rest/InferenceAPI.java) | Chọn model, gọi AI có authentication, giữ normalize result và lỗi. | Không | [Step 15](step/step-15-ai-abnormal-detection.md) |
| UC-011 — AI phân đoạn | [SegmentationAPI + ai](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/ai/rest/SegmentationAPI.java) | Giữ create/status/statistics/result, tải file atomic và đường dẫn cho 3D. | Chỉ nếu DICOM validation cần | [Step 16](step/step-16-ai-segmentation.md) |
| UC-012 — Quản lý model AI | [ModelController + ai](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/ai/rest/ModelController.java) | Giữ CRUD, model_name unique, URL và authentication/config JSON. | Không | [Step 6](step/step-06-ai-model-registry.md) |
| UC-013 — In DICOM | [PrinterAPI + printer](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/printer/rest/PrinterAPI.java) | Rust quản lý printer và validate request; Java thực hiện trọn giao dịch in DIMSE. | DICOM Print | [Step 17](step/step-17-dicom-print.md) |
| UC-014 — License và người dùng đồng thời | [LicenseStatusAPI + LicenseCcuAPI](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/license/rest/LicenseCcuAPI.java) | Giữ license gate, event-channel/heartbeat, fingerprint deduplication và CCU limit. | Không | [Step 7](step/step-07-license-ccu.md) |

Các class trong bảng là điểm bắt đầu đọc code; cần đối chiếu cả service, repository, job và test liên quan. Cấu hình quản trị nằm thêm trong [AdminAPI](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/util/rest/AdminAPI.java); trạng thái license nằm trong [LicenseStatusAPI](../../1pacs-viewer-core/src/main/java/vn/saolasoft/viewer/module/license/rest/LicenseStatusAPI.java).

**Ngoài phạm vi migration:** Rust không sử dụng logic seal. Không port `SealRuntimeAPI`, handshake `/api/rt`, runtime chunk `/api/rc/{*chunkPath}` hoặc service/crypto/runtime-session phục vụ seal. Các step loại seal khỏi phạm vi đối chiếu; Step 18 rà soát đầy đủ các endpoint/job còn lại.

## 3. Những phần nền tảng chuyển cùng tính năng

| Java Spring hiện tại | Rust tương ứng |
|---|---|
| Spring Boot/MVC, DTO và error response | Summer-rs/Axum, Serde, giữ HTTP contract và OpenAPI |
| WebRequestFilter, Spring Security, permission evaluator | Middleware auth/session và permission guard; xây trước CRUD cần phân quyền |
| JPA/PostgreSQL | Repository Rust với SeaORM trên DB riêng; giữ schema/type/constraint theo baseline |
| Session/config cache và scheduled jobs | Cache và scheduler Rust; giữ expiry, invalidation, retry và cleanup |
| OpenFeign/HTTP proxy | HTTP client Rust; giữ authentication, timeout, media/header và streaming |
| WebSocket và Python process pool | Rust quản lý kết nối, process, port, timeout và cleanup |
| dcm4che/ImageIO/DIMSE | Java facade không Spring, Rust gọi qua JNI |

Dữ liệu cần mapping gồm `SESSION`, `CONFIG`, `CLIENT`, `PERMISSION`, `CLIENT_PERMISSION`, `STUDY_KEY_IMAGE`, `MODELS`, `PRINTER`. Tham chiếu [entity model](../../1pacs-viewer-core/docs/entity_model.md); không tự thêm bảng/cột/quan hệ ngoài phạm vi được thống nhất.

## 4. Cách chuyển một tính năng

1. **Đọc Java:** xác định endpoint, service, dữ liệu, permission, job và tác động bên ngoài; đối chiếu [use case](../../1pacs-viewer-core/docs/use_cases/).
2. **Chốt hành vi cần giữ:** request/response, status/header, lỗi, DB effect và các luồng nghiệp vụ; lấy fixture đã loại PHI từ baseline.
3. **Viết phần tương ứng trong Rust:** giữ contract; chỉ gọi JNI cho phần DICOM đã xác định trong bảng.
4. **Kiểm tra tương đương:** chạy main flow, alternative flow, business rule, permission/error và kết quả dữ liệu/file; so với expected result từ Java.
5. **Cập nhật step:** ghi đã làm, việc còn lại, test/evidence và blocker. Chỉ đánh dấu `done` khi exit gate đạt; sau đó chuyển tính năng tiếp theo.

Nếu code Java khác tài liệu use case/entity, ghi discrepancy và xác minh trước phần implementation liên quan. Không tự sửa nghiệp vụ trong lúc port. Tính năng chưa đạt gate không được công bố là đã hỗ trợ.

## 5. Thứ tự triển khai và theo dõi

**Giữ nguyên 20 file step hiện tại.** Bảng dưới chỉ dẫn tới công việc chi tiết; trạng thái, checklist và checkpoint nằm trong từng file. Quy tắc cập nhật tại [docs/step/README.md](step/README.md).

| Step | Công việc |
|---|---|
| <a id="step-0"></a>Step 0 | [Nguồn tham khảo và quy tắc đối chiếu theo tính năng](step/step-00-baseline.md) |
| <a id="step-1"></a>Step 1 | [Khởi tạo Rust application](step/step-01-rust-application.md) |
| <a id="step-2"></a>Step 2 | [Kết nối PostgreSQL và nền tảng persistence](step/step-02-persistence.md) |
| <a id="step-3"></a>Step 3 | [Khởi tạo JVM/JNI và facade tối thiểu](step/step-03-jvm-jni-foundation.md) |
| <a id="step-3a"></a>Step 3a | [Auth/session foundation](step/step-03a-auth-session-foundation.md) |
| <a id="step-4"></a>Step 4 | [UC-009 Quản trị client và quyền](step/step-04-client-permission.md) |
| <a id="step-5"></a>Step 5 | [UC-008 Quản lý cấu hình Viewer](step/step-05-viewer-config.md) |
| <a id="step-6"></a>Step 6 | [UC-012 Quản lý model AI](step/step-06-ai-model-registry.md) |
| <a id="step-7"></a>Step 7 | [UC-014 Theo dõi license và người dùng đồng thời](step/step-07-license-ccu.md) |
| <a id="step-8"></a>Step 8 | [UC-001 Khởi tạo phiên xem ảnh 2D](step/step-08-create-session.md) |
| <a id="step-9"></a>Step 9 | [UC-002 Quản lý và chia sẻ phiên 2D](step/step-09-session-lifecycle.md) |
| <a id="step-10"></a>Step 10 | [UC-003 Tra cứu study DICOM](step/step-10-qido-study.md) |
| <a id="step-11"></a>Step 11 | [UC-004 Truy xuất dữ liệu ảnh DICOM](step/step-11-wado-retrieval.md) |
| <a id="step-12"></a>Step 12 | [UC-005 Tải nội dung DICOM](step/step-12-dicom-download.md) |
| <a id="step-13"></a>Step 13 | [UC-006 Tiếp nhận và xuất key image](step/step-13-key-image.md) |
| <a id="step-14"></a>Step 14 | [UC-007 Quản lý phiên dựng hình 3D](step/step-14-session-3d.md) |
| <a id="step-15"></a>Step 15 | [UC-010 Phân tích bất thường bằng AI](step/step-15-ai-abnormal-detection.md) |
| <a id="step-16"></a>Step 16 | [UC-011 Phân đoạn ảnh bằng AI](step/step-16-ai-segmentation.md) |
| <a id="step-17"></a>Step 17 | [UC-013 In ảnh qua máy in DICOM](step/step-17-dicom-print.md) |
| <a id="step-18"></a>Step 18 | [Hardening, full regression và hoàn tất phạm vi migration](step/step-18-hardening.md) |

Bắt đầu Step 1 ngay: init theo stack đã chốt, Step 2 kết nối DB riêng, Step 3 smoke test JVM/facade. Không chờ baseline Java toàn hệ thống hoặc mapping đủ tám entity. Step 0 chỉ giữ tham khảo đã có. Từ Step 3a, mỗi step tự đối chiếu Java, tạo entity/migration cần thiết và kiểm tra đúng tính năng của mình. Step 18 rà soát toàn hệ thống để phát hiện phần thiếu và kiểm thử tích hợp; không cản phát hành sớm tính năng đã đạt yêu cầu.

## 6. Các yêu cầu kỹ thuật cần giữ

- Khi JNI được bật: một JVM mỗi Rust process; readiness kiểm tra facade và codec của tính năng đang bật. Init chưa có JNI không bị chặn bởi JVM. Lỗi JVM không phục hồi xử lý bằng restart process.
- JNI chạy trên worker riêng có queue giới hạn; không block Tokio. Không chia sẻ `JNIEnv` giữa thread.
- Khi có Java exception: lấy `Throwable`, clear exception rồi mới đọc message/stack trace; xử lý cả lỗi thứ cấp và redact trước log.
- HTTP timeout không hủy Java call. Worker giữ buffer/file/permit/slot tới khi Java kết thúc; job đã hết hạn trong queue không được chạy.
- Print timeout sau khi có thể đã gửi lệnh là kết quả chưa xác định; không tự retry gây in trùng.
- Đóng gói đúng JVM/JAR/native codec, kiểm thử DICOM interoperability, resource limit và bảo mật. Không ghi PHI/credential vào log hoặc fixture.

Chi tiết đã thống nhất được giữ tại [Tham chiếu kỹ thuật JVM/JNI](SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md). **Các số mục cũ được nhắc trong file step** (2.2, 2.3, 3, 5–7, 9–13, 18…) hiện trỏ về số mục trong tài liệu tham chiếu này; không phải số mục của bản plan rút gọn.

**Công nghệ đã chốt:** Summer-rs/Axum, SeaORM, OpenAPI tích hợp Summer-rs, summer-job; Tokio/Serde/reqwest/moka/tracing/tokio-tungstenite. Production Linux x86_64, dev Windows, Java 25; dependency Java giữ baseline. Facade JAR + dependency directory, jni-rs, method riêng từng operation; byte[] nhỏ/file path lớn. Embedded JVM lỗi không phục hồi thì restart process; đổi kiến trúc phải hỏi lại.

Xem [bảng T1–T13 đã chốt](SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#decisions) và [điểm còn mở](SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#open-decisions): vendor/runtime packaging, phiên bản crate, T14 và P1–P4. Không tự điền tham số vận hành.
