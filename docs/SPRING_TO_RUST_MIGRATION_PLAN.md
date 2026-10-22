> **ĐÃ ĐƯỢC THAY THẾ — KHÔNG DÙNG TRIỂN KHAI.** Tài liệu lịch sử của phương án GraalVM/C ABI. Kiến trúc hiện tại là [plan JVM/JNI](SPRING_TO_RUST_JVM_JNI_MIGRATION_PLAN.md), database riêng, phát hành từng tính năng và loại seal. Mọi quyết định/step/cutover bên dưới không còn hiệu lực; giữ nội dung chỉ để tham khảo lịch sử.

# Kế hoạch chuyển đổi 1PACS Viewer Core từ Spring Java sang Rust

| Thuộc tính | Giá trị |
|---|---|
| Trạng thái | Draft - cần Tech Lead và QA phê duyệt |
| Phạm vi | Chuyển hành vi backend từ repo nguồn `1pacs-viewer-core` sang repo đích độc lập `1pacs-viewer-core-rust` |
| Nền tảng hiện tại | Java 25, Spring Boot 4.1, PostgreSQL, dcm4che, Orthanc Transcode, FFmpeg, Python/Trame/VTK |
| Nền tảng đích | Rust, Summer-rs/Axum, Tokio, PostgreSQL, dcm4che Native Shared Library, Python/Trame/VTK |
| Chiến lược | Xây dựng lại trong repo Rust độc lập theo milestone; Java chỉ là baseline tham chiếu, không chạy song song với Rust |
| Quyết định đã xác nhận | Các thư viện/chức năng liên quan dcm4che được giữ bằng GraalVM Native Image shared library và gọi từ Rust qua C ABI |

## 1. Mục tiêu

Chuyển backend từ Spring Java sang Rust nhưng giữ nguyên hành vi quan sát được của hệ thống, contract với frontend, schema PostgreSQL, giao tiếp PACS, DICOM/DICOMweb/DIMSE, DICOM Print và luồng render 3D.

Mục tiêu kỹ thuật:

1. Loại bỏ Spring/JVM khỏi application layer sau khi migration hoàn tất.
2. Giảm thời gian khởi động, memory baseline và độ phức tạp khi đóng gói application layer.
3. Giữ dcm4che làm DICOM engine để bảo toàn độ tương thích DICOM và thiết bị printer.
4. Giữ Python/Trame/VTK làm 3D rendering engine trong phạm vi kế hoạch này.
5. Xây dựng và kiểm thử Rust độc lập trong repo `1pacs-viewer-core-rust`; không triển khai Java và Rust song song, không chuyển traffic theo từng endpoint.
6. Không thay đổi API hoặc database schema nếu chưa có quyết định riêng được phê duyệt.

## 2. Ngoài phạm vi

- Viết lại Python/Trame/VTK bằng Rust.
- Thay dcm4che bằng một DICOM implementation thuần Rust.
- Thay đổi frontend viewer hoặc protocol frontend nếu không bắt buộc.
- Thiết kế lại toàn bộ database schema trong cùng đợt migration.
- Thay đổi business rule, permission hoặc session semantics chỉ vì framework mới.
- Tối ưu hoặc loại bỏ Orthanc/FFmpeg trước khi có kết quả compatibility benchmark.

## 3. Hiện trạng cần bảo toàn

Repo hiện có khoảng 262 file Java production và 47 file test. Các khu vực chính gồm:

| Module | Trách nhiệm chính | Mức rủi ro migration |
|---|---|---:|
| `qido` | Proxy truy vấn study/series/instance | Thấp |
| `ai` | Registry và proxy AI inference | Thấp-trung bình |
| `util` | Client/config/permission CRUD | Trung bình |
| `session` | Session cache + PostgreSQL + permission | Cao |
| `wado` | Streaming WADO-RS, metadata, frame và bulk data | Cao |
| `stowrs` | Multipart parsing, validation và tạo DICOM | Cao |
| `keyimage` | Tạo, lưu và gửi key image | Cao |
| `file` | Download, render, transcode, ZIP và video | Rất cao |
| `printer` | DIMSE và DICOM Print | Rất cao |
| `session3d` | Process pool và WebSocket bridge tới Trame | Cao |
| `license` / `seal` | License, runtime policy và sealed resources | Cao |

Các contract phải được giữ nguyên:

- Context path `/ws/rest/` và URL công khai của API/WebSocket.
- HTTP method, path, query parameter, header, status code và response body.
- Cấu trúc `WrapperResponse`/error response mà frontend đang sử dụng.
- Cách lấy session ID từ URL path, `session` hoặc `sessionId` query parameter.
- Basic Auth cho `/client/**`.
- Session Auth cho các request còn lại.
- Permission được cache trong session và `ADMIN` bypass.
- Short-term session lookup trước, long-term PostgreSQL fallback sau.
- DICOM media type, multipart boundary, transfer syntax và streaming behavior.
- Database schema hiện hữu, bao gồm kiểu PostgreSQL như `text[]`.
- Lifecycle của Python/Trame process và WebSocket binary/text frame.

## 4. Kiến trúc đích

```text
Frontend Viewer
       |
       v
Rust / Summer-rs application
  |-- REST + OpenAPI
  |-- Auth/session/permission middleware
  |-- PostgreSQL repositories
  |-- Local cache
  |-- HTTP clients tới PACS/AI/license services
  |-- WebSocket bridge
  |-- Scheduler/background jobs
  |-- Python/Trame process pool
  |
  +-- Safe Rust DICOM wrapper
        |
        v
      C ABI
        |
        v
  dcm4che GraalVM Native Shared Library
  |-- DICOM dataset/tag/VR/UID
  |-- DICOM JSON
  |-- Multipart/validation helpers
  |-- DIMSE/DICOM Print
  |-- ImageIO/codec/transcoding adapter
  +-- Native OpenCV/Orthanc dependencies khi cần
```

Rust là chủ sở hữu application lifecycle. dcm4che native shared library là DICOM kernel, không chứa Spring controller, database repository hoặc business orchestration.

## 5. Mapping công nghệ Spring sang Rust

| Spring/Java hiện tại | Rust đích | Lưu ý |
|---|---|---|
| Spring Boot lifecycle | `summer::App` | Khóa version chính xác trong `Cargo.lock` |
| Spring Web MVC | `summer-web`/Axum | Streaming body và multipart cần test riêng |
| Spring DI | Summer component hoặc explicit constructor | Ưu tiên dependency rõ ràng, tránh macro quá mức |
| `application.properties` | TOML + environment variables | Có lớp compatibility cho tên config cũ trong giai đoạn chuyển tiếp |
| Spring Data JPA | SeaORM hoặc SQLx | SeaORM cho CRUD; SQLx cho truy vấn đặc thù/streaming |
| Hibernate/HikariCP | SeaORM/SQLx PostgreSQL pool | Không tự động dựa vào lazy loading |
| OpenFeign | `reqwest` | Tự định nghĩa timeout, redirect, TLS và header forwarding |
| Spring Security filter | Tower/Axum middleware | Port chính xác Basic Auth + session auth semantics |
| `@PreAuthorize` | Permission extractor/guard | Permission name phải giữ nguyên |
| Spring Cache/Caffeine | `moka` hoặc cache service riêng | Không dùng Redis nếu chưa có yêu cầu multi-instance |
| Spring WebSocket | Axum WS + `tokio-tungstenite` | Có backpressure và giới hạn frame |
| `@Scheduled` | `summer-job` hoặc Tokio task | Phải chống job chạy trùng khi nhiều replica |
| Spring Async/virtual threads | Tokio task + bounded blocking pool | Không block Tokio worker bằng FFI/codec/filesystem |
| Jackson/Gson | Serde/`serde_json` | Golden test cho null, enum, timestamp và field naming |
| Springdoc | Summer OpenAPI/`utoipa` | OpenAPI đích phải diff với baseline |
| SLF4J/Logback | `tracing` | Correlation ID và PHI-safe logging |
| JUnit/Mockito/MockMvc | Rust unit/integration tests | Dùng test case, fixture và expected result đã trích xuất từ Java làm baseline tĩnh |
| GraalVM executable app | Rust native executable | GraalVM chỉ còn dùng cho dcm4che shared library |

## 6. Thiết kế dcm4che Native Shared Library

### 6.1 Ranh giới trách nhiệm

Shared library nên cung cấp operation coarse-grained:

- Parse DICOM thành DICOM JSON.
- Serialize DICOM JSON/dataset thành DICOM.
- Đọc/ghi/validate tag, VR, UID và transfer syntax.
- Parse/validate multipart DICOM khi việc giữ dcm4che là cần thiết.
- Render/decode frame.
- Transcode DICOM.
- Tạo key image DICOM.
- Thực hiện trọn một DICOM Print job.
- Các DIMSE operation khác chỉ khi repo thực sự sử dụng.

Không expose trực tiếp Java object, `Attributes`, `Association`, `Sequence` hoặc exception sang Rust. Không chia một print job thành nhiều lời gọi FFI làm state của association đi qua ABI.

### 6.2 C ABI

ABI chỉ dùng kiểu C ổn định: integer có độ rộng rõ ràng, pointer, length và struct `repr(C)`.

```c
typedef struct {
    uint32_t abi_version;
    uint32_t struct_size;
    const uint8_t *data;
    size_t length;
} pacs_input_buffer_t;

typedef struct {
    int32_t status;
    uint8_t *data;
    size_t length;
    char *error_message;
} pacs_result_t;

uint32_t pacs_dicom_abi_version(void);
int32_t pacs_dicom_parse_json(...);
int32_t pacs_dicom_validate(...);
int32_t pacs_dicom_transcode(...);
int32_t pacs_dicom_render_frame(...);
int32_t pacs_dicom_create_key_image(...);
int32_t pacs_dicom_print(...);
void pacs_dicom_result_free(pacs_result_t *result);
```

Yêu cầu bắt buộc:

- ABI có version và kiểm tra version khi Rust khởi động.
- Shared library cấp phát thì shared library phải giải phóng.
- Không để Java exception hoặc Rust panic vượt qua FFI boundary.
- Mỗi operation trả error code ổn định và error message đã loại PHI.
- Có giới hạn input size/output size và kiểm tra overflow.
- Pointer null, length sai và UTF-8 không hợp lệ phải trả lỗi, không crash.
- Rust raw bindings nằm trong crate `dicom-native-sys`; safe wrapper nằm trong crate `dicom-native`.
- Application code không được gọi trực tiếp `unsafe extern "C"`.

### 6.3 Isolate và threading

- Tạo một GraalVM isolate trong lifecycle của Rust process, trừ khi benchmark chứng minh cần nhiều isolate.
- Không chuyển `graal_isolatethread_t*` giữa các OS thread.
- Dùng tập native worker thread cố định; mỗi worker attach vào isolate và chỉ xử lý FFI trên chính thread đó.
- Tokio request gửi job qua bounded channel và nhận kết quả qua oneshot channel.
- Không gọi DICOM FFI blocking trực tiếp trên Tokio async worker.
- Có concurrency limit riêng cho parse, render, transcode và print.
- Shutdown phải ngừng nhận job, đợi job đang chạy theo timeout, detach thread rồi teardown isolate.

### 6.4 Dữ liệu lớn

- Metadata/control message có thể dùng UTF-8 JSON.
- DICOM instance nhỏ có thể truyền bằng byte buffer.
- Study/series/video lớn ưu tiên file path trong workspace tạm đã kiểm soát.
- Không base64 pixel data hoặc DICOM file trong JSON.
- File path phải được canonicalize và giới hạn trong thư mục cho phép.
- Tất cả temporary file phải có owner, TTL và cleanup job.

### 6.5 Packaging

Tạo artifact riêng cho từng OS/architecture:

```text
native/linux-x86_64/libpacs_dicom.so
native/windows-x86_64/pacs_dicom.dll
native/<platform>/pacs_dicom.h
native/<platform>/manifest.json
```

Manifest tối thiểu chứa ABI version, Git commit, GraalVM version, dcm4che version, enabled codecs, target OS/architecture và checksum.

Container/runtime phải đóng gói đủ dependent native libraries như OpenCV, Orthanc Transcode, libc/libstdc++ hoặc các codec tương ứng. CI phải kiểm tra missing symbol và dynamic dependency trước khi publish.

## 7. Cấu trúc Rust workspace đề xuất

```text
rust/
├── Cargo.toml
├── Cargo.lock
├── crates/
│   ├── viewer-core/          # application bootstrap
│   ├── viewer-api/           # routes, DTO, OpenAPI
│   ├── viewer-domain/        # domain model và business rule
│   ├── viewer-persistence/   # PostgreSQL repositories
│   ├── viewer-auth/          # Basic/session/permission middleware
│   ├── viewer-dicomweb/      # QIDO/WADO/STOW HTTP clients
│   ├── viewer-session3d/     # process pool + WebSocket bridge
│   ├── dicom-native/         # safe wrapper
│   └── dicom-native-sys/     # generated/raw FFI bindings
└── tests/
    ├── contract/
    ├── compatibility/
    └── fixtures/
```

Không bắt buộc giữ layer/package giống Spring. Giữ module theo capability và dependency direction:

```text
API -> application/domain -> ports <- persistence/http/ffi adapters
```

## 8. Chiến lược migration

### Giai đoạn 0 - Baseline và compatibility harness

Mục tiêu: đóng băng hành vi hiện tại trước khi viết Rust.

- Export OpenAPI hiện tại.
- Lập inventory tất cả endpoint và permission.
- Ghi baseline status/header/body cho happy path và error path.
- Chuẩn bị DICOM golden corpus đã loại PHI.
- Ghi database schema/checksum và migration hiện tại.
- Ghi benchmark baseline: startup, RSS, request latency, throughput, transcode time và memory peak.
- Chụp response/expected result từ Java thành fixture tĩnh có version và loại bỏ PHI.
- Tạo compatibility test runner chạy Rust rồi so sánh với baseline đã lưu, không yêu cầu Java chạy cùng lúc.

Exit gate:

- Có API inventory và OpenAPI baseline.
- Có bộ fixture đủ transfer syntax và DIMSE/DICOM Print scenario quan trọng.
- Có đầy đủ baseline để kiểm thử Rust độc lập trong repo mới.

### Giai đoạn 1 - dcm4che shared library proof of concept

Thực hiện tối thiểu:

1. Build dcm4che thành Native Image shared library.
2. Rust tạo isolate và gọi một entrypoint.
3. Parse DICOM -> JSON.
4. Transcode về Explicit VR Little Endian.
5. Render một frame.
6. Thực hiện một print job với printer test hoặc simulator.
7. Chạy concurrent stress test và shutdown test.

Exit gate:

- Không crash/leak trong soak test đã thống nhất.
- Output dataset/transfer syntax tương đương Java baseline.
- Error/timeout có thể quan sát và không làm kẹt Tokio runtime.
- ABI/version mismatch bị từ chối rõ ràng.

### Giai đoạn 2 - Rust application skeleton

- Bootstrap Summer-rs/Axum.
- Configuration, structured logging và health/readiness endpoint.
- PostgreSQL connection pool.
- Error model và response envelope tương thích.
- OpenAPI generation.
- Shutdown lifecycle gồm HTTP, database, worker và native isolate.

Exit gate: service khởi động, readiness đúng, graceful shutdown đúng và OpenAPI contract cơ bản pass.

### Giai đoạn 3 - Module rủi ro thấp

Thứ tự đề xuất:

1. `qido`.
2. AI HTTP proxy.
3. Config/client/printer CRUD, chưa chuyển print execution.
4. Các license/status endpoint thuần HTTP sau khi xác nhận contract.
5. Scheduled cleanup đơn giản.

Mỗi endpoint chỉ được đánh dấu hoàn thành khi contract/compatibility tests pass. Chưa triển khai production theo từng endpoint trong giai đoạn này.

### Giai đoạn 4 - Session, authentication và authorization

- Port Basic Auth cho `/client/**`.
- Port session ID extraction từ path/query.
- Port short-term cache và long-term PostgreSQL fallback.
- Port expiry/delete behavior.
- Port permission cache và admin bypass.
- So sánh permission decision của Rust với permission matrix/expected result đã ghi từ Java baseline.
- Đảm bảo không log credential, session token hoặc PHI.

Exit gate: toàn bộ auth/session/permission matrix pass, gồm request thiếu session, session hết hạn, permission thiếu và admin bypass.

### Giai đoạn 5 - WADO/QIDO/STOW/key image

- WADO streaming không buffer toàn bộ response nếu Java hiện đang stream.
- Header forwarding phải lọc hop-by-hop header.
- Bảo toàn `Content-Type`, multipart boundary và transfer syntax parameter.
- Xử lý disconnect/backpressure và upstream timeout.
- STOW multipart parser phải chống payload quá lớn và malformed boundary.
- Validation/tạo key image gọi dcm4che shared library.
- So sánh DICOM dataset ở mức tag/VR/value/pixel checksum, không chỉ byte-for-byte khi encoding hợp lệ có thể khác.

### Giai đoạn 6 - File, codec và media export

- Port download instance/series/study và ZIP structure.
- Gọi dcm4che shared library cho DICOM transcode/render.
- Port image/video export với bounded blocking/concurrency.
- Giữ FFmpeg CLI/native integration ban đầu nếu đã ổn định.
- Kiểm tra cancellation, disk-full, temporary-file cleanup và memory peak.

### Giai đoạn 7 - DICOM Print và DIMSE

- Rust quản lý request/API/DB; native shared library thực hiện toàn bộ print job.
- Một FFI call đại diện một print transaction hoàn chỉnh.
- Timeout print phải được giới hạn; vì không thể hủy tùy ý native call, chạy print trên worker riêng và có circuit breaker.
- Kiểm tra presentation context, association reject, DIMSE status, printer offline và malformed image.
- Nếu native DIMSE call có nguy cơ treo/crash không thể kiểm soát, ngoại lệ được phép là chuyển riêng DICOM Print sang native executable sidecar; quyết định này cần ADR.

### Giai đoạn 8 - 3D session và WebSocket

- Giữ Python/Trame/VTK.
- Port pre-warmed process pool, port allocation, health check và watchdog.
- Port frontend WebSocket server và outgoing WebSocket client.
- Bảo toàn text/binary frame và wslink behavior.
- Thêm backpressure, frame size limit, connection timeout và cleanup idempotent.
- Test process crash, websocket reconnect, client disconnect trước attach và server shutdown.

### Giai đoạn 9 - License/seal và release cutover

- Port license/startup cache/CCU/heartbeat theo contract hiện hữu.
- Port sealed-resource behavior nếu vẫn cần trong Rust distribution.
- Hoàn tất kiểm thử trong môi trường staging độc lập với production Java.
- Dừng Java deployment trong maintenance window và triển khai Rust như một release thay thế hoàn chỉnh.
- Không chia traffic, không shadow request và không vận hành Java/Rust đồng thời.
- Rollback production, nếu cần, là khôi phục nguyên release Java cũ; không route qua lại theo endpoint.
- Chỉ loại bỏ khả năng phục hồi release Java sau khi Rust vượt qua thời gian ổn định đã thống nhất.

## 9. Chiến lược kiểm thử

### 9.1 Contract tests

Mỗi endpoint phải kiểm tra:

- Method/path/query/header.
- Authentication và permission.
- Status code.
- Response header.
- JSON shape, nullability và field naming.
- Streaming/multipart behavior.
- Error code/message.

### 9.2 Compatibility tests với baseline Java

Trước khi dừng phát triển Java, chụp request, response, database effect và expected result thành fixture có version. Test chỉ chạy Rust candidate và so sánh normalized output với fixture. Cho phép danh sách khác biệt được phê duyệt, ví dụ header thời gian hoặc multipart boundary ngẫu nhiên; không bỏ qua khác biệt mặc định.

### 9.3 DICOM compatibility matrix

Golden corpus tối thiểu:

- Explicit/Implicit VR Little Endian.
- JPEG Baseline/Lossless, JPEG-LS, JPEG 2000 và RLE đang được hỗ trợ.
- 8/12/16-bit, signed/unsigned.
- MONOCHROME1/MONOCHROME2/RGB/YBR phù hợp dữ liệu thực tế.
- Single-frame và multi-frame.
- Encapsulated pixel data.
- Missing/invalid optional attributes.
- Large study/series.

Validation:

- Tag, VR, multiplicity và value.
- SOP/Study/Series UID rule.
- Transfer syntax.
- Frame count/dimensions.
- Pixel checksum sau decode khi lossless.
- Tolerance/quality metric đã thống nhất khi lossy.
- Khả năng mở output bằng ít nhất một PACS/viewer độc lập.

### 9.4 FFI tests

- Null pointer và zero length.
- Invalid UTF-8/JSON/DICOM.
- Buffer quá lớn và integer overflow.
- Concurrent calls.
- Repeated allocation/free.
- Rust cancellation khi job đang chạy.
- Worker panic/native error.
- Init/shutdown lặp lại trong test process riêng.
- ABI mismatch và thiếu symbol.

### 9.5 Non-functional tests

- Startup time và readiness.
- RSS idle và peak memory.
- Throughput/latency cho QIDO/WADO.
- Backpressure khi client chậm.
- Transcode/render concurrency.
- Database pool exhaustion.
- File descriptor/thread/temp-file leak.
- Soak test.
- Graceful shutdown trong lúc có HTTP, WebSocket, print và transcode job.

## 10. Security và dữ liệu y tế

- Không ghi Patient Name, Patient ID, accession number, DICOM dataset hoặc pixel data vào log mặc định.
- Redact Authorization, Basic credentials, session ID và signed URL.
- Giới hạn request/multipart/frame size trước khi cấp phát buffer.
- Validate path và chống path traversal cho file operation/ZIP.
- Không tin MIME type hoặc filename từ client/upstream.
- Timeout cho HTTP, DB, WebSocket, FFI worker và DIMSE.
- Dependency/SBOM scan cho Cargo, GraalVM artifact và native libraries.
- Shared library chỉ được load từ đường dẫn cấu hình tin cậy và kiểm tra checksum/version.
- Rust `unsafe` chỉ tồn tại trong crate FFI và được review riêng.

## 11. Observability

Mỗi request/job có correlation ID xuyên qua Rust, FFI và external services. Metric tối thiểu:

- HTTP request count, latency và error theo route/status.
- Upstream PACS/AI/license latency và failure.
- DB pool utilization.
- Session cache hit/miss/eviction.
- FFI queue depth, active workers, duration và error theo operation.
- Bytes input/output cho WADO/STOW/transcode.
- Print job duration và DIMSE status.
- 3D pool idle/borrowed/process restart/WebSocket connection.
- Temporary storage usage và cleanup failure.

Native library không được tự do ghi stdout không cấu trúc. Cần callback/log bridge hoặc log format có component, operation và correlation ID.

## 12. Deployment và rollback

Java và Rust không được triển khai song song. Repo `1pacs-viewer-core-rust` có pipeline build/test/release riêng và được kiểm thử trong môi trường staging độc lập.

Quy tắc release cutover:

1. Toàn bộ contract, compatibility, DICOM interoperability, security và non-functional gate pass trên Rust.
2. Backup database/configuration và xác minh restore procedure.
3. Dừng Java deployment trong maintenance window.
4. Thực hiện database migration tương thích theo release plan.
5. Triển khai toàn bộ Rust release.
6. Chạy smoke test/readiness và kiểm tra domain metric.
7. Mở lại traffic khi gate sau triển khai đạt yêu cầu.

Rollback production không phải cơ chế chạy hai implementation. Nếu cutover thất bại:

1. Dừng Rust release.
2. Rollback database bằng procedure đã được kiểm chứng, hoặc giữ schema backward-compatible.
3. Khôi phục nguyên Java release cũ và configuration tương ứng.
4. Chạy smoke test trước khi mở lại traffic.

Không thực hiện route switch theo endpoint, shadow traffic hoặc canary chia traffic giữa Java và Rust.

Rollback trigger tối thiểu:

- Tăng error rate vượt ngưỡng được phê duyệt.
- DICOM output không tương thích.
- Permission decision sai.
- Memory/thread/file descriptor leak.
- Print/DIMSE timeout hoặc thiết bị reject tăng bất thường.
- Không thể quan sát hoặc chẩn đoán lỗi production.

## 13. Rủi ro và biện pháp kiểm soát

| ID | Rủi ro | Kiểm soát |
|---|---|---|
| RISK-001 | Summer-rs còn ở version `0.x` hoặc API thay đổi | Khóa version/Cargo.lock; facade framework; upgrade có test |
| RISK-002 | C ABI lỗi ownership gây crash/leak | Safe wrapper; sanitizer/Valgrind tương ứng; soak test; review `unsafe` |
| RISK-003 | Graal isolate thread dùng sai trên Tokio | Dedicated native worker threads; không chia sẻ isolate-thread pointer |
| RISK-004 | Native call treo, không hủy được | Timeout ở queue/worker; circuit breaker; cân nhắc sidecar cho print |
| RISK-005 | Khác biệt JSON/API làm frontend lỗi | OpenAPI + contract + compatibility tests từ baseline tĩnh |
| RISK-006 | Khác biệt DICOM encoding/codec | Golden corpus; tag/pixel validation; PACS/viewer interoperability |
| RISK-007 | Async Rust bị block bởi DB/file/FFI | Bounded blocking pool; profiling; concurrency limit |
| RISK-008 | Dual-run xử lý job hai lần | Ownership flag/DB lock/leader election/idempotency |
| RISK-009 | Dependency native thiếu trên target | Per-platform manifest; container smoke test; symbol/dependency scan |
| RISK-010 | PHI/credential xuất hiện trong log | Central redaction; log tests; production log policy |
| RISK-011 | Database semantics khác Hibernate | Integration tests trên PostgreSQL thật; explicit transaction/isolation |
| RISK-012 | WebSocket/process cleanup race | State machine; idempotent cleanup; reconnect/crash tests |
| RISK-013 | License/seal semantics thay đổi | Port cuối; golden contract; controlled rollout |

## 14. Điều kiện hoàn thành migration

Migration chỉ được xem là hoàn thành khi:

- Toàn bộ endpoint trong inventory đã cut over hoặc có ngoại lệ được phê duyệt.
- OpenAPI/contract tests và permission matrix pass.
- DICOM/DICOM Print interoperability suite pass.
- FFI stress/soak/leak test pass theo ngưỡng được Tech Lead và QA duyệt.
- Performance không thấp hơn ngưỡng được thống nhất.
- Production observability, alert và runbook đã có.
- Rollback drill đã thực hiện thành công.
- Java không còn nhận production traffic trong thời gian ổn định đã thống nhất.
- Database schema không còn phụ thuộc vào migration chỉ Java hiểu.
- SBOM/license/security review hoàn tất.
- Tech Lead, QA và owner vận hành chấp nhận cutover cuối.

## 15. Deliverable theo giai đoạn

| Giai đoạn | Deliverable chính |
|---|---|
| 0 | API inventory, OpenAPI baseline, golden corpus, benchmark, compatibility harness |
| 1 | dcm4che shared library, C header, Rust safe wrapper, FFI test report |
| 2 | Rust skeleton, config, DB, health, logging, OpenAPI |
| 3 | QIDO/AI/CRUD endpoints và compatibility report |
| 4 | Auth/session/permission implementation và decision matrix |
| 5 | WADO/STOW/key image implementation và DICOM compatibility report |
| 6 | Download/transcode/media export và resource benchmark |
| 7 | DICOM Print/DIMSE interoperability report |
| 8 | 3D process pool/WebSocket implementation và resilience report |
| 9 | License/seal, release cutover report, rollback evidence và operations handoff |

## 16. Quyết định cần phê duyệt trước khi triển khai

1. Summer-rs là framework bắt buộc hay Axum trực tiếp là fallback được phép?
2. SeaORM, SQLx hoặc kết hợp hai loại cho persistence?
3. OS/architecture nào phải được hỗ trợ chính thức?
4. ABI v1 gồm chính xác operation nào?
5. DICOM Print dùng shared library trong-process hay được phép tách sidecar nếu soak test thất bại?
6. Ngưỡng performance, memory, error rate và thời gian soak test?
7. Thời gian giữ Java release artifact và database restore point sau cutover?
8. Maintenance window và thời gian rollback tối đa cho full-release cutover?
9. Bộ PACS/printer/viewer độc lập nào được dùng làm interoperability oracle?
10. Ai là Tech Lead, QA owner, Security owner và Operations owner phê duyệt từng gate?

## 17. Bước tiếp theo đề xuất

1. Tech Lead review và chốt các quyết định tại mục 16.
2. Tạo endpoint/API inventory tự động từ Spring controllers và OpenAPI.
3. Chuẩn bị DICOM corpus đã loại PHI và printer/PACS test environment.
4. Thực hiện Giai đoạn 1 dưới dạng proof of concept độc lập.
5. Chỉ mở rộng migration sau khi FFI, codec và DICOM Print exit gate đạt yêu cầu.
