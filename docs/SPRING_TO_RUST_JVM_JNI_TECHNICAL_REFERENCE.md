# Tham chiếu kỹ thuật — Rust JVM/JNI

Tài liệu triển khai theo các quyết định người dùng chốt ngày 2026-10-22 tại mục 18; các tham số còn mở được ghi riêng, không coi là đã duyệt. [Plan chính](SPRING_TO_RUST_JVM_JNI_MIGRATION_PLAN.md) trình bày ánh xạ tính năng; tài liệu này dùng khi triển khai chi tiết. Số mục cũ được giữ để các file step hiện tại tiếp tục tham chiếu chính xác. Không quản lý tiến độ tại đây.

**Phạm vi cập nhật:** Rust không sử dụng logic seal; toàn bộ API và logic phục vụ seal được loại khỏi migration theo plan chính. Các yêu cầu bảo toàn tính năng/endpoint bên dưới chỉ áp dụng trong phạm vi này.

## 1. Mục tiêu

Chuyển application layer từ Spring Java sang Rust trong repo `1pacs-viewer-core-rust`, đồng thời tiếp tục sử dụng dcm4che dưới dạng Java bytecode chạy trong một JVM được Rust khởi tạo và điều khiển qua JNI.

Mục tiêu cụ thể:

1. Loại bỏ Spring Boot, Spring MVC, Spring Data, Spring Security và Spring Cloud khỏi runtime mới.
2. Giữ dcm4che và hành vi DICOM/DIMSE/DICOM Print hiện tại thay vì port sang DICOM library thuần Rust.
3. Không build dcm4che/application facade bằng GraalVM Native Image.
4. Rust sở hữu HTTP, authentication, session, permission, PostgreSQL, cache, scheduler, WebSocket và process orchestration.
5. JVM chỉ là DICOM runtime nội bộ, không chạy Spring application context và không mở HTTP port.
6. Giữ contract với frontend, PACS, printer, database và Python/Trame/VTK.
7. Kiểm thử Rust bằng OpenAPI, compatibility fixture và expected result tĩnh lấy từ Java baseline; Rust dùng database riêng và có thể triển khai từng nhóm tính năng độc lập với Spring BE.

**Cách triển khai:** init Rust và kết nối DB không cần baseline Java toàn hệ thống. Đối chiếu code/UC/entity và thu fixture tại step tính năng liên quan. Step 3 chỉ JVM/facade smoke test; kiểm thử operation DICOM tại step sử dụng; tổng coverage tại Step 18.

## 2. Kiến trúc đích

```text
Frontend Viewer
       |
       v
Rust / Summer-rs application
  |-- REST + OpenAPI
  |-- Auth/session/permission
  |-- PostgreSQL repositories
  |-- Cache/scheduler/background jobs
  |-- HTTP clients tới PACS/AI/license services
  |-- WebSocket + Python/Trame process pool
  |
  +-- dicom-jvm Rust adapter
        |-- JVM lifecycle
        |-- JNI worker pool
        |-- exception/reference/buffer management
        |
        v
     JNI Invocation API
        |
        v
  Embedded JVM (một JVM/process)
        |
        v
  pacs-dicom-facade.jar
        |-- dcm4che dataset/tag/VR/UID
        |-- DICOM JSON
        |-- ImageIO/codec/transcoding
        |-- DIMSE/DICOM Print
        +-- multipart/validation/key image helpers
```

Rust gọi `JNI_CreateJavaVM` gián tiếp qua crate JNI có hỗ trợ Invocation API. JVM được load từ `libjvm.so`, `jvm.dll` hoặc `libjvm.dylib` thuộc Java runtime được đóng gói hoặc cấu hình.

### 2.1 Nguồn chức năng và dữ liệu bắt buộc

Kế hoạch và implementation trong repo Rust phải bám theo hai nguồn sau của repo Java:

- [Danh mục use case](../../1pacs-viewer-core/docs/use_cases/): UC-001 đến UC-014, gồm precondition, main flow, alternative flow, postcondition và business rule.
- [Entity model](../../1pacs-viewer-core/docs/entity_model.md): `SESSION`, `CONFIG`, `CLIENT`, `PERMISSION`, `CLIENT_PERMISSION`, `STUDY_KEY_IMAGE`, `MODELS`, `PRINTER`.

Source code Spring hiện tại là nguồn đối chiếu kỹ thuật và hành vi legacy. Khi code khác với use case hoặc entity model, không tự chọn một phía: ghi nhận discrepancy, xác minh với owner rồi cập nhật tài liệu nguồn trước khi triển khai Rust.

### 2.2 Rules bắt buộc cho toàn bộ migration

1. **Use-case-first:** Không triển khai endpoint hoặc service nghiệp vụ nếu chưa gắn với ít nhất một `UC-xxx`, hoặc ghi rõ đây là hạ tầng dùng chung.
2. **Bảo toàn đầy đủ luồng:** Definition of Done của một use case phải bao phủ main success scenario, mọi alternative flow, success/failure postcondition và business rule trong file use case tương ứng.
3. **Entity-model-first:** Repository, migration, entity/row mapping và validation phải bám đúng entity model. Không tự tạo quan hệ, foreign key, cascade hoặc cột mới chỉ từ suy luận.
4. **Traceability trong code:** Tên work item, PR, test suite hoặc test module phải chứa `UC-xxx` đối với thay đổi nghiệp vụ. Test phải chỉ ra business rule/alternative flow được kiểm chứng.
5. **Một use case, một gate:** Chỉ chuyển sang use case kế tiếp khi step hiện tại đạt exit gate; hạ tầng dùng chung có thể được mở rộng nhưng không được coi là hoàn tất thay cho use case.
6. **Không đưa business logic vào JNI facade:** Rust sở hữu validation nghiệp vụ, auth, session, permission, transaction và orchestration. Java facade chỉ cung cấp thao tác dcm4che/DICOM/DIMSE coarse-grained.
7. **Ghi rõ phạm vi JNI:** Mỗi step phải chỉ ra `Không dùng JNI` hoặc liệt kê operation JNI cụ thể. Không gọi JNI cho CRUD, auth, session, config, license hoặc HTTP proxy thuần túy.
8. **Tham chiếu ngắn gọn:** Step chỉ ghi công việc và dẫn trực tiếp tới quyết định/thiết kế liên quan trong tài liệu này; không lặp lại quy tắc công nghệ chung.
9. **Contract theo tính năng:** DTO, status/header/media/error và DB effect được chốt tại step của use case trước khi port; không thu contract toàn backend trước init. Entity/migration được tạo khi tính năng cần.
10. **Triển khai độc lập, database riêng:** Java cung cấp baseline tĩnh cho kiểm thử. Rust không dùng chung database với Spring BE và được triển khai lần lượt theo tính năng đã đạt gate; không cần dừng hoặc thay thế Spring BE khi phát hành Rust. Các chức năng chưa hoàn thành không được công bố là đã hỗ trợ.
11. **Không thay đổi schema ngầm:** Thay đổi entity/schema phải cập nhật `entity_model.md`, migration plan, compatibility fixture và được review trước code.
12. **Không bỏ qua PHI/security:** Fixture phải loại PHI; log không chứa credential, session, DICOM metadata nhạy cảm hoặc pixel data.
13. **JNI fail-fast:** JVM/facade API/capability mismatch làm readiness fail. Pending Java exception phải được map và clear trước JNI call tiếp theo.
14. **Bằng chứng hoàn thành:** Mỗi step phải để lại contract tests, integration tests, evidence về expected database state và metric/benchmark nếu có NFR liên quan.

### 2.3 Ma trận use case và entity

| Use case | Capability | Entity đọc/ghi | JVM/JNI |
|---|---|---|---|
| UC-001 | Khởi tạo phiên xem ảnh 2D | `CLIENT`, `PERMISSION`, `CLIENT_PERMISSION`, `SESSION`, `CONFIG` | Không |
| UC-002 | Quản lý và chia sẻ phiên 2D | `SESSION`, `CONFIG` | Không |
| UC-003 | Tra cứu study DICOM | `SESSION` | Không bắt buộc; dùng HTTP QIDO-RS |
| UC-004 | Truy xuất dữ liệu ảnh DICOM | `SESSION` | Chỉ dùng khi cần parse/normalize DICOM JSON; proxy streaming do Rust |
| UC-005 | Tải nội dung DICOM | `SESSION` | Transcode/render DICOM qua Java facade |
| UC-006 | Tiếp nhận và xuất key image | `SESSION`, `STUDY_KEY_IMAGE`, `CONFIG` | Validate/tạo/ghi DICOM qua Java facade |
| UC-007 | Quản lý phiên dựng hình 3D | `SESSION`; phiên 3D là runtime registry | Không cho orchestration; DICOM engine chỉ dùng gián tiếp nếu cần tiền xử lý |
| UC-008 | Quản lý cấu hình Viewer | `CONFIG` | Không |
| UC-009 | Quản trị client và quyền | `CLIENT`, `PERMISSION`, `CLIENT_PERMISSION` | Không |
| UC-010 | Phân tích bất thường bằng AI | `MODELS` | Không; HTTP tới AI service |
| UC-011 | Phân đoạn ảnh bằng AI | `MODELS`, `SESSION`; kết quả lưu filesystem | Không bắt buộc; HTTP/file orchestration do Rust |
| UC-012 | Quản lý model AI | `MODELS` | Không |
| UC-013 | In ảnh qua máy in DICOM | `PRINTER`, `SESSION` | Có; dcm4che/DIMSE Print transaction trong Java facade |
| UC-014 | Theo dõi license và người dùng đồng thời | Không có entity bền vững trong entity model; runtime state/config | Không |

<a id="toml-config"></a>

### 2.4 Contract cấu hình vận hành

Quyết định người dùng ngày 2026-10-26, sau khi xác nhận phạm vi: dùng TOML
cho các thành phần đã triển khai, **giữ nguyên cơ chế nạp cấu hình của Summer**.
Không vá/fork framework và không thêm CLI chọn file mới.

#### Nạp cấu hình và thứ tự ưu tiên

- Giữ `App::new()`: Summer nạp `config/app.toml`, sau đó overlay cùng thư mục
  theo profile (`app-dev.toml`, `app-test.toml`, `app-prod.toml`). Profile mặc
  định dev, `SUMMER_ENV` chọn profile như framework hiện có.
- Giữ cơ chế `.env`/nội suy `${NAME}` và `${NAME:default}` của Summer. Không
  thêm loader dotenv hoặc tự nội suy lần thứ hai trong code ứng dụng.
- Giữ `RUST_LOG` ghi đè logging. `[web]`, `[logger]`, OpenAPI, job và shutdown
  dùng framework/dependency hiện tại, không thay đổi default bootstrap.
- Sau khi framework nạp/merge, `AppConfig::from_registry()` bind `[database]`,
  `[auth]`, `[session]`, `[jni]` từ **cùng registry**. Module không tự đọc
  `VIEWER_*`/`PACS_*` hay gọi `from_env()` để lấy giá trị cấu hình.
- Env chỉ có tác dụng khi framework sử dụng nó, ví dụ placeholder trong TOML,
  profile/logging. Literal trong file overlay không bị module đọc env ghi đè.
- Không triển khai `--config`, không yêu cầu một file độc lập thay thế overlay.
  Chạy từ root repo để tìm `config/`. Đường dẫn JNI tương đối vẫn tính theo cwd
  như Step 3; không đổi sang thư mục file TOML.

#### File mẫu và chạy local

Mẫu duy nhất thêm mới: [config/app-dev.example.toml](../config/app-dev.example.toml).
Sao chép thành `config/app-dev.toml` (đã gitignore), sửa URL/user/password DB và
Basic admin. File mẫu dùng literal, local không cần khai báo từng biến env.

```powershell
Copy-Item config/app-dev.example.toml config/app-dev.toml
# Edit config/app-dev.toml, then:
cargo run --locked -p viewer-app
```

`config/app.toml` chứa cấu hình nền và placeholder env tùy chọn; `app-prod.toml`
vẫn là overlay production hiện có. File thật có secret không commit; deploy
có thể mount thư mục config/profile như trước. Giữ nguyên snapshot baseline Java.

```toml
[database]
url = "postgresql://127.0.0.1:5432/viewer_rust"
username = "postgres"
password = "REPLACE_WITH_LOCAL_DB_PASSWORD"
max_connections = 10
timeout_seconds = 5

[auth]
client_security_disabled = false
admin_username = "sysadmin"
admin_password = "REPLACE_WITH_LOCAL_ADMIN_PASSWORD"

[session]
default_expire_minutes = 60
path_prefix = "/viewer?session="

[jni]
enabled = false
```

#### Schema và validation ứng dụng

| Khóa | Contract |
|---|---|
| `database.url` | Bắt buộc: PostgreSQL URL có host/database, không nhúng credential; dùng hai trường riêng. |
| `database.username`, `database.password` | Chuỗi bắt buộc có trong cấu hình hiệu lực, giữ nguyên giá trị; code dùng URL setters để encode ký tự đặc biệt. |
| `database.max_connections` | Số nguyên dương, default 10; min pool giữ 1. |
| `database.timeout_seconds` | Số nguyên dương, default 5; giữ phạm vi connect/acquire/readiness/migration hiện tại. |
| `auth.client_security_disabled` | Boolean, default false; không đổi Basic/IP/admin/session/permission business rules. |
| `auth.admin_username`, `auth.admin_password` | Chuỗi, giữ default `sysadmin` của implementation trước; cấu hình explicit trong file local/deploy. |
| `session.default_expire_minutes` | i64, default 60; validate overflow timestamp, giữ normalize expireIn như Java. Step 8/9 nhận expireIn dạng i32 hoặc integer string (trim/dấu; blank thành null); sai kiểu/ngoài i32 bị từ chối trước ghi dữ liệu. |
| `session.path_prefix` | Chuỗi, default `/viewer?session=`; CONFIG PATH_PREFIX trong DB vẫn ưu tiên cùng cache/invalidation Step 5/8. |
| `jni.enabled` | Boolean, default false; false không yêu cầu runtime/heap/timeout. |
| `web`, `logger` | Schema/default và env/profile override của Summer hiện tại, không fork framework. |

Các section ứng dụng từ chối unknown key/sai kiểu; lỗi binding/DB/JNI chỉ trả
category an toàn, không in raw deserialize/driver error hay config Debug.
Pool, migration, hashing, API/error business và session lifecycle không đổi.
Cú pháp TOML, interpolation và thông báo lỗi của bộ nạp Summer giữ nguyên.

Khi JNI bật, các khóa nằm trong cùng section `[jni]`:

| Khóa | Quy tắc giữ từ Step 3 |
|---|---|
| `java_home`, `libjvm_path` | Ít nhất một, cả hai phải trỏ cùng Java 25; không dò JDK trên PATH. |
| `facade_dir` | Distribution JAR/dependencies/manifest/checksum hợp lệ. |
| `temp_dir`, `native_dir` | Thư mục đã tồn tại, path tương đối theo cwd. |
| `xms_mb`, `xmx_mb`, `direct_mb` | Số nguyên dương khi bật; Xms ≤ Xmx; không chốt sizing production P1–P4. |
| `timeout_ms`, `shutdown_ms` | Số nguyên dương khi bật; giữ lifecycle/fatal recovery. |
| `check_jni` | Boolean tùy chọn; default debug=true, release=false; test đặt true. |

Các khóa JNI có ví dụ comment trong file mẫu. Adapter nhận `JvmSettings` từ
ứng dụng rồi validate thành `JvmConfig`, không phụ thuộc Summer hoặc đọc env
để lấy runtime settings. Vẫn từ chối `JAVA_TOOL_OPTIONS`, `_JAVA_OPTIONS`,
`JDK_JAVA_OPTIONS` không rỗng để ngăn option ngoài cấu hình. PATH/JAVA_HOME của
OS/build tool không phải nguồn cấu hình nghiệp vụ hoặc cơ chế chọn JVM mới.

#### Đối chiếu implementation cũ

| Nguồn cũ | Khóa TOML |
|---|---|
| VIEWER_DATABASE_URL/USER/PASSWORD | database.url/username/password |
| VIEWER_DATABASE_MAX_CONNECTIONS/TIMEOUT_SECONDS | database.max_connections/timeout_seconds |
| VIEWER_CLIENT_SECURITY_DISABLED | auth.client_security_disabled |
| VIEWER_ADMIN_USERNAME/PASSWORD | auth.admin_username/admin_password |
| VIEWER_DEFAULT_EXPIRE_MINUTES/PATH_PREFIX | session.default_expire_minutes/path_prefix |
| VIEWER_JNI_ENABLED | jni.enabled |
| PACS_JAVA_HOME/LIBJVM_PATH | jni.java_home/libjvm_path |
| PACS_FACADE_DIR | jni.facade_dir |
| PACS_JVM_TEMP_DIR/NATIVE_DIR | jni.temp_dir/native_dir |
| PACS_JVM_XMS_MB/XMX_MB/DIRECT_MB | jni.xms_mb/xmx_mb/direct_mb |
| PACS_JVM_TIMEOUT_MS/SHUTDOWN_MS/CHECK_JNI | jni.timeout_ms/shutdown_ms/check_jni |

Tên env cũ không còn được module ứng dụng đọc trực tiếp. Có thể dùng chúng
trong placeholder TOML nếu cần tương thích vận hành. `VIEWER_CONFIG` không
còn là đường chọn file riêng của ứng dụng; giữ đường nạp mặc định/profile
của framework theo xác nhận người dùng. SUMMER_ENV và RUST_LOG giữ nguyên.

#### Kiểm chứng refactor

Test unit kiểm tra binding/default/type/unknown key/overflow và giữ nguyên
credential. Integration PostgreSQL hồi quy migration/transaction/auth/config/
create-session trên DB riêng. Smoke dùng thư mục tạm với `config/app.toml` và
profile overlay, kiểm tra HTTP thật, interpolation, RUST_LOG override và việc
module không đọc biến cũ ngoài TOML. Không sửa registry hoặc framework khi test.

CI tạo `config/app-dev.toml` từ mẫu với credential fixture riêng. Python harness
đọc local TOML literal để tạo fixture, không cung cấp settings qua env ứng dụng.
`JNI_TEST_CONFIG` chỉ truyền đường dẫn file TOML cho libtest (không có CLI app);
`JNI_TEST_CASE` điều khiển artifact gây lỗi. Cả hai chỉ phục vụ test, không có
nhánh production đọc chúng để tạo session hoặc chọn runtime. Secret fixture
không upload. Kết quả thực tế được ghi tại mục cập nhật bên dưới sau khi chạy.

#### Kết quả kiểm chứng 2026-10-26

Refactor đã triển khai trên Windows local; không thay version hoặc fork Summer.
File mẫu mới: `config/app-dev.example.toml`. Không sửa snapshot Java. Thay đổi
tài liệu Step 6 có từ trước không làm tăng tiến độ implementation của step.
Người dùng đã xác nhận commit toàn bộ thay đổi và push ngày 2026-10-26.

- `cargo fmt --all -- --check` và `cargo clippy --workspace --all-targets --locked -- -D warnings`: đạt.
- `cargo test --workspace --locked`: 7 unit đạt; các test cần DB/JNI không được tính lượt skip là pass.
- Lệnh PostgreSQL regression trong DEVELOPMENT_COMMANDS: 5 test đạt, 0 skip; hồi quy migration/readiness/transaction và Step 3a/4/5/8.
- Debug/release workspace build và `python tools/smoke_app.py` (cả `--release`): đạt dev/prod, HTTP session/link/config, startup validation và shutdown. Test riêng xác nhận overlay/profile, interpolation và RUST_LOG override; biến cũ không được module đọc khi fixture TOML dùng literal.
- `python tools/smoke_jni.py --java-home 'C:/Program Files/Eclipse Adoptium/jdk-25.0.4.101-hotspot' --app` và cùng lệnh thêm `--release`: đạt cả hai bản, gồm subprocess suite, PostgreSQL/JVM readiness, HTTP dev/prod, startup failure, fatal recovery và shutdown.
- Kiểm tra cú pháp file mẫu/snippet TOML, Python harness, liên kết tài liệu và `git diff --check`: đạt. CI đã sửa để tạo profile TOML test; workflow Windows/Ubuntu chưa chạy trong lượt này.

Evidence không commit: `target/config-toml-logs/` có `workspace-tests.log`,
`postgres-regression.log`, `format.log`, `build-debug.log`, `build-release.log`,
`smoke-debug.log`, `smoke-release.log`, `jni-debug.log`, `jni-release.log`.
Các thông số JVM/DB tạm chỉ phục vụ test, không chốt sizing production.
PostgreSQL tạm đã dừng/xóa; trạng thái cuối CLIENT/SESSION/CONFIG/mapping đều
0 bản ghi, không còn kết nối app. Profile local chứa credential test đã xóa,
chỉ giữ file mẫu. Evidence bổ sung: `final-database-state.log`, `postgres-stop.log`.

Nguồn theo framework đã khóa:
[Summer configuration](https://summer-rs.github.io/docs/getting-started/config/).
Không thêm summer-sea-orm/plugin DB thứ hai để đổi nguồn cấu hình.

## 3. Các quyết định kiến trúc bắt buộc

### 3.1 Một JVM duy nhất trong mỗi Rust process

- Chỉ tạo một JVM cho mỗi process.
- Khi tính năng JNI được bật, JVM khởi tạo trước readiness; bản init chưa dùng JNI không cần JVM. Codec/operation chỉ bắt buộc khi tính năng tương ứng được bật.
- Nếu JVM khởi tạo thất bại, readiness fail và application không nhận traffic.
- Không thiết kế restart JVM in-process. Nếu JVM rơi vào trạng thái không phục hồi, process Rust phải thoát để orchestrator/container restart toàn bộ.
- Không coi `DestroyJavaVM` là cơ chế recovery trong runtime bình thường.

### 3.2 Java facade thay vì gọi trực tiếp dcm4che từ mọi module Rust

Tạo một Maven module Java nhỏ, không phụ thuộc Spring:

```text
java/dicom-facade/
├── pom.xml
└── src/
    ├── main/java/vn/saolasoft/pacs/dicom/DicomFacade.java
    ├── main/java/vn/saolasoft/pacs/dicom/DicomResult.java
    ├── main/java/vn/saolasoft/pacs/dicom/PrintRequest.java
    └── test/java/...
```

Facade dùng bộ dependency Java hiện tại làm baseline (dcm4che 5.33.1 theo pom.xml nguồn); không nâng dependency trong đợt port. Chỉ lấy dependency cần cho DICOM, không mang Spring sang.

Facade cung cấp operation coarse-grained:

- `parseToJson(byte[])`
- `validate(byte[])`
- `transcode(byte[], options)` hoặc `transcodeFile(input, output, options)`
- `renderFrame(byte[], frameIndex, options)`
- `createKeyImage(request)`
- `print(PrintRequest)`
- Các operation DIMSE khác chỉ khi repo thực sự sử dụng.

Không gọi dcm4che theo kiểu một JNI call cho mỗi tag hoặc mỗi method nhỏ. Không truyền `Attributes`, `Association`, `Sequence`, `BufferedImage` hoặc Java collection qua application boundary.

### 3.3 API facade ổn định và có version

Facade phải cung cấp:

```java
public final class DicomFacade {
    public static int apiVersion();
    public static String capabilitiesJson();
    // Method riêng cho parse/validate/render/transcode/create-key-image/print.
    // Signature và DTO cụ thể được xác định trong Step 3.
}
```

Đã chốt dùng các static method riêng cho từng operation, không dùng dispatcher execute(operation, request). Contract request/response phải có version độc lập với version dcm4che; signature/DTO là đầu ra thiết kế của Step 3.

Quy tắc:

- `apiVersion()` được Rust kiểm tra khi startup.
- `capabilitiesJson()` liệt kê codec/operation được bật.
- Domain error được chuyển thành response envelope ổn định.
- Java exception chỉ dành cho lỗi lập trình/JVM không mong đợi.
- Không để Rust phụ thuộc vào tên class nội bộ hoặc method signature của dcm4che.

## 4. Mapping Spring sang Rust

| Hiện tại | Rust đích | Lưu ý |
|---|---|---|
| Spring Boot lifecycle | Summer-rs/Axum bootstrap | JVM là dependency nội bộ của Rust lifecycle |
| Spring MVC | `summer-web`/Axum | Bảo toàn REST/streaming/multipart contract |
| Spring Data JPA | SeaORM | Dùng SeaORM cho persistence; không bổ sung lớp SQLx trực tiếp |
| Spring Security | Tower/Axum middleware | Port Basic/session/permission semantics |
| `@PreAuthorize` | Permission guard/extractor | Permission constant giữ nguyên |
| Spring Cache/Caffeine | `moka` | Session cache behavior phải tương thích |
| OpenFeign | `reqwest` | Timeout/TLS/header forwarding explicit |
| Spring WebSocket | Axum WS + `tokio-tungstenite` | Có backpressure/frame limit |
| Commons Pool 2 | Tokio channel/semaphore/custom pool | Dùng cho Python process và JNI workers |
| `@Scheduled` | `summer-job` | Không để job chạy trùng |
| Jackson/Gson | Serde | Java facade có thể dùng Jackson riêng cho request nhỏ |
| Springdoc | Tích hợp OpenAPI của Summer-rs | Diff với OpenAPI baseline |
| SLF4J/Logback | `tracing` phía Rust; SLF4J bridge phía JVM | Correlation ID xuyên JNI |
| dcm4che | Giữ nguyên trên JVM | Không Native Image |
| Orthanc/OpenCV/FFmpeg Java bindings | Giữ khi cần | Native dependency của Java vẫn phải đóng gói |

## 5. Thiết kế Rust JNI adapter

### 5.1 Crate structure

```text
rust/crates/
├── dicom-jni-sys/       # JNI signatures/raw interop tối thiểu
├── dicom-jvm/           # safe JVM lifecycle + facade wrapper
├── viewer-dicom/        # DicomEngine trait/application adapter
└── viewer-core/         # Summer-rs application
```

Dùng crate `jni`/jni-rs; phiên bản và feature chính xác cần được kiểm tra, khóa trong Cargo.lock tại Step 3. Đoạn trait dưới đây minh họa ranh giới nghiệp vụ, chưa phải signature đã chốt cho byte/file và async dispatch.

Chỉ `dicom-jvm` được thao tác `JNIEnv`, `JObject`, `JClass`, method signature và Java reference. Các module nghiệp vụ chỉ thấy interface an toàn:

```rust
pub trait DicomEngine: Send + Sync {
    fn parse_json(&self, input: &[u8]) -> Result<Vec<u8>, DicomError>;
    fn validate(&self, input: &[u8]) -> Result<ValidationResult, DicomError>;
    fn transcode(&self, request: TranscodeRequest) -> Result<Vec<u8>, DicomError>;
    fn render_frame(&self, request: RenderRequest) -> Result<Vec<u8>, DicomError>;
    fn create_key_image(&self, request: KeyImageRequest) -> Result<Vec<u8>, DicomError>;
    fn print(&self, request: PrintRequest) -> Result<PrintResult, DicomError>;
}
```

### 5.2 JVM initialization

Khởi tạo JVM với cấu hình explicit:

```text
-Djava.class.path=<facade.jar + dependency classpath>
-Djava.awt.headless=true
-Djava.io.tmpdir=<controlled temp directory>
-Djava.library.path=<native codec directory>
-Xms<approved minimum>
-Xmx<approved maximum>
-XX:MaxDirectMemorySize=<approved limit>
-XX:+ExitOnOutOfMemoryError
```

Trong development bật `-Xcheck:jni`. Production không bật nếu overhead không chấp nhận được, nhưng phải có một test profile luôn chạy với tùy chọn này.

Không tự động sử dụng JDK bất kỳ trên host production. Runtime phải chỉ tới Java distribution đã kiểm thử qua `jni.java_home`/`jni.libjvm_path` trong TOML đã chọn, theo contract cấu hình chung.

### 5.3 Thread attach/detach

`JNIEnv` chỉ hợp lệ trên thread đã attach và không được gửi sang thread khác.

Thiết kế đề xuất:

```text
Tokio request
    |
    v
bounded mpsc queue
    |
    v
fixed JNI worker threads
    |-- attach một lần vào JavaVM
    |-- xử lý synchronous JNI call
    |-- dùng local reference frame cho từng job
    +-- detach khi worker shutdown
```

Yêu cầu:

- Không gọi JNI blocking trực tiếp trên Tokio worker.
- Không lưu hoặc chia sẻ `JNIEnv` giữa các thread.
- `JavaVM` handle có thể được chia sẻ theo API của binding; `JNIEnv` thì không.
- Worker pool có queue bound và timeout.
- Tách pool `print/DIMSE` khỏi pool `parse/render/transcode` nếu latency khác biệt lớn.
- Không spawn một OS thread mới cho mỗi request.

### 5.4 Class, method và object reference

- Resolve facade class/method trong startup smoke test.
- Cache class dưới dạng global reference, không giữ local reference qua call boundary.
- Cache method ID nếu binding hỗ trợ an toàn và signature đã được kiểm tra.
- Mỗi job dùng local frame có giới hạn để giải phóng local reference sau call.
- Loop xử lý series/frame phải xóa local reference định kỳ.
- Global reference phải có lifecycle rõ ràng và được giải phóng trước JVM shutdown.
- Không lưu raw Java object pointer trong domain/application structs.

<a id="jni-exceptions"></a>

### 5.5 Exception handling

Cơ sở: [Java 25 JNI — Exception Handling](https://docs.oracle.com/en/java/javase/25/docs/specs/jni/design.html#exception-handling).

Sau mỗi JNI method call:

1. Kiểm tra JNI error/pending exception.
2. Lấy local reference tới `Throwable` bằng `ExceptionOccurred`; chưa gọi Java method để đọc exception.
3. Gọi `ExceptionClear` trước khi lấy class, message hoặc bounded stack trace từ `Throwable`.
4. Nếu việc đọc thông tin phát sinh exception mới, check/clear exception đó và dùng error fallback tối thiểu; không đệ quy thu thập stack trace. Giải phóng reference trong local frame của job.
5. Map sang `DicomError` có error code ổn định.
6. Redact PHI/credential/path nhạy cảm trước khi log; không dùng `ExceptionDescribe` để ghi trực tiếp exception chưa redact ra stderr trong production.

Facade nên bắt exception dự kiến và trả `DicomResult`; Rust chỉ xử lý pending Java exception như lỗi bất thường.

### 5.6 Byte array, direct buffer và file

Ba cơ chế truyền dữ liệu:

| Cơ chế | Dùng khi | Lưu ý |
|---|---|---|
| `byte[]` | Metadata hoặc DICOM nhỏ | Có copy Rust -> JVM và JVM -> Rust |
| Direct `ByteBuffer` | Buffer lớn, cần giảm copy | Lifetime phải bảo đảm; Java không giữ buffer sau call |
| File path | Study/series/transcode/video lớn | Canonicalize path và giới hạn sandbox directory |

Quy tắc:

- POC bắt đầu bằng `byte[]` để giảm rủi ro correctness.
- Đã chốt T12: bắt đầu bằng `byte[]` cho dữ liệu nhỏ và file path cho dữ liệu lớn; ngưỡng chưa chốt (P3). Direct buffer chưa triển khai ban đầu; chỉ xem xét sau benchmark chứng minh cần thiết.
- Không giữ `GetPrimitiveArrayCritical` qua I/O hoặc call kéo dài.
- Không base64 DICOM/pixel data trong JSON.
- File operation dùng thư mục tạm riêng, quota, TTL và cleanup.

<a id="jni-ownership"></a>

### 5.7 Timeout, cancellation và ownership

- Phân biệt deadline chờ queue, deadline chờ kết quả và thời điểm Java call thực sự kết thúc. Worker kiểm tra deadline/cancellation trước khi bắt đầu job; job đã hết hạn hoặc bị hủy trong queue không được chạy.
- Khi worker nhận job, job phải sở hữu input/output buffer, direct-buffer backing memory và file lease độc lập với HTTP request. Request timeout/disconnect chỉ dừng việc chờ kết quả; không giải phóng tài nguyên mà Java còn sử dụng.
- Giữ slot worker, memory permit và file lease đến khi Java call thực sự trả về. Không trả slot sớm hoặc tạo worker thay thế không giới hạn để che giấu call bị treo.
- Chỉ cleanup file/output và giải phóng backing memory sau khi Java kết thúc và không còn consumer sử dụng; TTL cleanup phải bỏ qua file đang được lease.
- Cooperative cancellation là best effort. Call không phục hồi phải theo cơ chế recovery cấp process; queue vẫn có giới hạn và từ chối tải mới khi quá tải.
- Print timeout sau khi có thể đã gửi N-ACTION được phân loại `outcome_unknown` nội bộ, không tự động retry. Mapping ra HTTP phải theo contract được duyệt; không báo thành công giả hoặc khẳng định chưa in. Cần đối soát trước khi gửi lại.
- Test bắt buộc: hết hạn trong queue không gọi facade; disconnect khi Java còn chạy; buffer/file/permit còn hợp lệ đến khi call kết thúc; cleanup sau completion; print timeout không tự retry.

## 6. Thiết kế Java dcm4che facade

### 6.1 Không có Spring dependency

Facade JAR không chứa:

- Spring Boot/application context.
- Controller, repository hoặc security filter.
- PostgreSQL client.
- Scheduled job.
- Business session/cache.

Facade chỉ chứa dcm4che adapter, DTO tối thiểu, validation và mapping error.

### 6.2 Packaging JAR

Đã chốt **facade JAR + thư mục dependency JAR**, với classpath đầy đủ; không dùng fat JAR. Kiểm thử META-INF/services, dictionary và ImageIO/codec resources trên bản đóng gói.

### 6.3 DICOM Print

Rust gửi một `PrintRequest` hoàn chỉnh. Java facade sở hữu toàn bộ:

- Device/ApplicationEntity/Connection.
- Association negotiation.
- Presentation context.
- N-CREATE/N-SET/N-ACTION.
- DIMSE response/status.
- Association release/abort.

Một print transaction không được chia thành nhiều JNI call giữ `Association` giữa các call. Print chạy trên JNI worker pool riêng với concurrency thấp.

JNI timeout chỉ giới hạn thời gian Rust chờ; nó không tự hủy được Java method đang block. Vì vậy Java facade phải cấu hình connect/read/DIMSE timeout và hỗ trợ cooperative cancellation nếu khả thi.

Ownership, cleanup và trạng thái print chưa xác định kết quả tuân theo mục 5.7.

### 6.4 Native codec dependency

Không build Native Image không có nghĩa là loại bỏ mọi native dependency. Nếu dcm4che ImageIO, OpenCV, Orthanc Transcode hoặc FFmpeg binding cần `.so`/`.dll`, các thư viện này vẫn phải:

- Được đóng gói theo OS/architecture.
- Có `java.library.path` rõ ràng.
- Được smoke-test khi startup.
- Có checksum/manifest/version.
- Được kiểm tra dynamic dependency trong CI.

## 7. Java runtime distribution

Đã chốt Java **25**, production **Linux x86_64**, môi trường phát triển **Windows**. Vendor/build cụ thể và JRE đầy đủ hay jlink còn mở; không tự chọn. Tạo runtime distribution có version cố định sau khi xác nhận các lựa chọn này:

```text
runtime/
├── java/                 # jlink runtime hoặc JRE distribution
├── java-libs/
│   ├── pacs-dicom-facade.jar
│   └── dependencies/*.jar
├── native-libs/
│   └── <platform>/*
└── manifest.json
```

Manifest ghi:

- Java vendor/version/build.
- dcm4che version.
- Facade API version và Git commit.
- Danh sách JAR/checksum.
- Native codec/checksum.
- Supported OS/architecture.
- JVM options mặc định.

CI phải tạo reproducible artifact cho từng target platform được hỗ trợ.

## 8. Các step triển khai

Xem [danh mục step](step/README.md). Trạng thái chỉ ghi tại file step tương ứng.

## 9. Chiến lược kiểm thử

<a id="facade-tests"></a>

### 9.1 Java facade tests

- Unit test dcm4che adapter trên JVM thông thường.
- Test facade API version/capabilities.
- Test domain error envelope.
- Test malformed DICOM, invalid transfer syntax và printer failure.
- Test ServiceLoader/ImageIO/codec resources từ packaged distribution.

### 9.2 JNI adapter tests

- JVM path/classpath sai.
- Class/method/signature không tồn tại.
- Pending Java exception.
- Local/global reference pressure.
- Attach/detach worker thread.
- Rust cancellation khi Java call đang chạy.
- Large `byte[]` và direct buffer lifetime.
- Concurrent calls.
- JVM OOM simulation trong test process riêng.
- Process shutdown với Java thread còn sống.

### 9.3 Contract/compatibility tests

- HTTP method/path/query/header/status/body.
- Authentication/permission.
- JSON field/null/enum/time semantics.
- Streaming/multipart.
- Database write/transaction effect.
- Error response.

Expected result là fixture có version trích từ Java baseline; không gọi live Java backend trong pipeline Rust.

### 9.4 DICOM matrix

- Explicit/Implicit VR Little Endian.
- JPEG Baseline/Lossless, JPEG-LS, JPEG 2000 và RLE đang hỗ trợ.
- 8/12/16-bit; signed/unsigned.
- MONOCHROME1/MONOCHROME2/RGB/YBR phù hợp dữ liệu thật.
- Single-frame/multi-frame/encapsulated pixel data.
- Missing/invalid optional attributes.
- Large study/series.

Kiểm tra tag, VR, multiplicity, UID, transfer syntax, frame metadata, decoded pixel checksum và interoperability với PACS/printer/viewer độc lập.

<a id="nfr-tests"></a>

### 9.5 Non-functional tests

- Rust startup bao gồm JVM startup/warm-up.
- RSS được tách theo Rust heap, JVM heap, direct/native memory.
- GC pause và throughput.
- JNI queue depth/latency.
- WADO streaming/backpressure.
- Transcode/render/print concurrency.
- Soak test và thread/reference/file descriptor/temp-file leak.
- Graceful shutdown trong lúc có HTTP, WebSocket, transcode và print job.

## 10. Memory và performance

Ngân sách memory phải bao gồm:

```text
Total RSS = Rust heap
          + JVM heap (-Xmx)
          + Java metaspace/code cache
          + Java direct buffers
          + Rust/native buffers
          + codec/OpenCV/FFmpeg native memory
          + thread stacks
```

Không chỉ theo dõi JVM heap. Metric tối thiểu:

- JVM heap used/committed/max.
- GC count/pause.
- Direct buffer usage.
- JNI active workers/queue/duration/error.
- Rust allocator memory.
- Process RSS/thread/file descriptor.

Tối ưu theo thứ tự:

1. Giảm số JNI calls bằng operation coarse-grained.
2. Bounded concurrency.
3. Dùng file path cho payload rất lớn.
4. DirectByteBuffer khi benchmark chứng minh cần thiết.
5. Điều chỉnh JVM heap/GC sau profiling, không dựa trên phỏng đoán.

## 11. Logging và observability

- Correlation ID được truyền trong facade request/context.
- Java log có component `dicom-jvm` và correlation ID.
- Không ghi Patient Name, Patient ID, accession, dataset hoặc pixel data mặc định.
- Redact Authorization, Basic credentials, session ID và filesystem path nhạy cảm.
- Có metric cho JVM init, class load, JNI call, Java exception, print/DIMSE status và codec availability.
- Capture bounded Java stack trace cho lỗi bất thường; không trả stack trace cho client.
- Khi JNI bật, readiness fail nếu JVM/facade hoặc capability của tính năng đang bật không hợp lệ; không đòi capability chưa triển khai.

## 12. Security

- Chỉ load `libjvm` và JAR từ đường dẫn cấu hình tin cậy.
- Kiểm tra checksum/manifest của Java distribution khi startup hoặc trong deployment verification.
- Không cho request điều khiển classpath, JVM option hoặc Java library path.
- File path gửi sang Java phải canonicalize và nằm trong allowed root.
- Giới hạn HTTP/multipart/JNI payload trước allocation.
- Scan CVE/license cho Cargo crates, JRE, dcm4che/JAR và native codecs.
- JNI/unsafe code chỉ nằm trong adapter crate và phải được review riêng.

## 13. Build và CI/CD

Pipeline gồm hai phần:

```text
Java facade build
  -> Maven test
  -> dependency/resource verification
  -> facade.jar + dependency jars

Rust build
  -> cargo fmt/clippy/test
  -> JNI integration tests với packaged JRE
  -> contract/DICOM tests
  -> assemble final distribution/container
```

CI matrix chạy trên từng OS/architecture được hỗ trợ. Final artifact phải bao gồm Rust executable, Java runtime, facade/dependency JAR, native codec và manifest/checksum.

Không dùng `JAVA_HOME` ngẫu nhiên từ build agent để chứng minh production compatibility. Integration test phải chạy trên đúng runtime distribution được đóng gói.

## 14. Deployment và rollback

Rust dùng database riêng và vận hành độc lập với Spring BE. Schema Java là nguồn tham chiếu compatibility, không phải database runtime dùng chung. Chuyển dữ liệu hoặc tác vụ đang chạy từ Java không nằm trong phạm vi kế hoạch này.

Phát hành từng nhóm tính năng:

1. Chốt danh sách use case được hỗ trợ trong release và dependency đã hoàn tất; chỉ bật route/tính năng đã đạt gate.
2. Chạy contract/integration/security regression cho phạm vi release và các tính năng Rust đã có trên staging.
3. Chuẩn bị database/config/storage riêng của Rust; kiểm tra migration và khả năng rollback về release Rust trước nếu có.
4. Deploy Rust + packaged JVM/JAR/native-codec distribution.
5. Startup smoke test kiểm tra JVM, facade API, codec và database; chạy smoke test nghiệp vụ thuộc phạm vi release.
6. Mở tính năng khi gate đạt yêu cầu. Không cần dừng Spring BE hoặc đợi đủ UC-001..UC-014.

Rollback giới hạn trong môi trường Rust: dừng hoặc vô hiệu hóa release/tính năng lỗi, quay về release Rust trước nếu có và dùng phương án database tương thích đã kiểm thử. Với release đầu, dừng tính năng lỗi và giữ dữ liệu để xử lý. Không restore database Java hoặc khôi phục Spring BE như một bước rollback Rust.

## 15. Rủi ro

| ID | Rủi ro | Kiểm soát |
|---|---|---|
| JVMJNI-R01 | `JNIEnv` được dùng sai thread | Fixed JNI workers; không share `JNIEnv`; attach/detach test |
| JVMJNI-R02 | Local/global reference leak | Local frames; bounded loop; stress/soak; global-ref lifecycle |
| JVMJNI-R03 | Pending Java exception làm hỏng call sau | Check/map/clear exception sau mỗi call |
| JVMJNI-R04 | JNI call quá nhỏ gây overhead | Coarse-grained facade operations |
| JVMJNI-R05 | Copy `byte[]` làm tăng memory/latency | Benchmark; file path/direct buffer khi cần |
| JVMJNI-R06 | Java call block làm cạn worker | Bounded pool/queue; Java timeout; circuit breaker |
| JVMJNI-R07 | Không thể restart JVM an toàn | Fail process và để orchestrator restart |
| JVMJNI-R08 | JAR/resource/ServiceLoader thiếu | Dependency directory hoặc shade transformer; startup capability test |
| JVMJNI-R09 | Sai JRE/libjvm trên production | Packaged runtime; explicit path; manifest/checksum |
| JVMJNI-R10 | Tổng RSS vẫn cao do nhúng JVM | Memory budget; Xmx/direct limits; benchmark với Java baseline theo P4 |
| JVMJNI-R11 | Java/native codec crash làm chết Rust | Soak test; guarded concurrency; restart process; đổi kiến trúc phải hỏi người dùng |
| JVMJNI-R12 | DICOM Print treo | Java DIMSE timeout; worker pool riêng; process-level recovery |
| JVMJNI-R13 | Khác API/database behavior | OpenAPI/contract/fixture compatibility tests |
| JVMJNI-R14 | PHI xuất hiện trong Java log/exception | Redaction facade + Rust log policy + tests |

## 16. Kiến trúc đã chốt

Giữ embedded JVM + JNI. Khi JVM/call không phục hồi, restart toàn bộ process Rust. Không tự chuyển sang sidecar hoặc GraalVM Native Shared Library. POC dùng để kiểm chứng kiến trúc đã chọn; nếu không đạt, báo evidence và hỏi người dùng trước khi thay đổi kiến trúc.

## 17. Điều kiện hoàn thành

- Rust endpoint inventory hoàn tất hoặc có ngoại lệ được duyệt.
- OpenAPI/contract/session/permission tests pass.
- JVM/JNI POC và production adapter pass `-Xcheck:jni`, stress và soak tests.
- DICOM/DICOM Print interoperability suite pass.
- Packaged JRE/JAR/native codec chạy trên mọi target platform.
- JVM heap/direct/native memory và total RSS trong ngưỡng được duyệt.
- JVM failure/exception/OOM/shutdown behavior đã được diễn tập.
- Security/SBOM/license review hoàn tất.
- Deployment và rollback runbook cho release Rust độc lập đã được thử trên staging.
- Tech Lead, QA và Operations chấp nhận release.

<a id="decisions"></a>

## 18. Quyết định công nghệ

Nguồn quyết định: người dùng xác nhận T1–T11 và T12–T13 trong hội thoại ngày 2026-10-22. Chốt lựa chọn không đồng nghĩa đã hoàn tất implementation hoặc kiểm thử.

| ID | Đã chốt |
|---|---|
| T1 | Summer-rs/Axum |
| T2 | SeaORM cho persistence |
| T3 | Tích hợp OpenAPI của Summer-rs |
| T4 | Tokio, Serde, reqwest, moka, tracing, tokio-tungstenite |
| T5 | summer-job |
| T6 | Production Linux x86_64; phát triển trên Windows |
| T7 | Java 25; vendor/build và cách đóng gói runtime chưa chốt |
| T8 | Giữ dependency Java hiện tại làm baseline; dcm4che 5.33.1 theo pom.xml nguồn |
| T9 | Facade JAR + thư mục dependency JAR |
| T10 | Crate jni/jni-rs; phiên bản cụ thể chưa khóa |
| T11 | Method riêng cho parse, validate, render, transcode, create-key-image, print; không dispatcher chung |
| T12 | byte[] cho dữ liệu nhỏ, file path cho dữ liệu lớn; ngưỡng chưa chốt; chưa dùng direct buffer ban đầu |
| T13 | Embedded JVM + restart process khi không phục hồi; đổi kiến trúc phải hỏi lại người dùng |

<a id="open-decisions"></a>

### Những điểm còn mở

| ID | Nội dung | Step liên quan |
|---|---|---|
| T7 bổ sung | Vendor/build Java 25; JRE đầy đủ hay jlink | 3, 18 |
| T14 | FFmpeg: giữ Java binding hay chuyển sang process do Rust quản lý | 12 |
| Version | Phiên bản Rust toolchain/crates và feature tương thích, khóa Cargo.lock; không tự coi bản latest là đã chốt | 1–3 |
| P1 | Số worker, dung lượng queue; print dùng pool riêng | 3, 17 |
| P2 | RAM/container, JVM heap/direct memory và tổng RSS | 3, 18 |
| P3 | Timeout, payload limit, ngưỡng byte[]/file và quota temp storage | 3, 12, 13, 17 |
| P4 | Ngưỡng so với Java baseline, thời gian soak và bộ PACS/printer/viewer kiểm chứng | 0, 3, 18 |

Theo yêu cầu người dùng, các tham số P1–P4 giữ **chưa chốt**. Có thể tiếp tục inventory và thiết kế không phụ thuộc chúng; trước khi chạy bài đo/test cần ghi cấu hình thử nghiệm, không coi số thử nghiệm là cấu hình production đã duyệt.

## 19. Bước tiếp theo

1. Bắt đầu Step 1–2: init Rust và kết nối PostgreSQL riêng; không đợi inventory/fixture Java. Giữ baseline Step 0 đã thu thập làm tham khảo.
2. Triển khai theo thứ tự 20 step, dùng lựa chọn T1–T13 đã chốt.
3. Step 3 xác định phiên bản JNI và smoke test JVM/facade tối thiểu. Signature/DTO, byte[]/file transport, worker limits và codec được hoàn thiện/test tại step tính năng sử dụng đầu tiên.
4. Từng step thu baseline/fixture đúng phạm vi và kiểm tra tương đương. Step 18 rà soát tổng coverage, stress/soak và hiệu năng; P1–P4 chưa chốt, không yêu cầu POC Native Shared Library.

<a id="technology-sources"></a>

## 20. Nguồn tham khảo công nghệ

- [Summer-rs — repository chính thức](https://github.com/summer-rs/summer-rs): đối chiếu tích hợp Axum, OpenAPI và summer-job theo phiên bản được khóa ở Step 1.
- [SeaORM — tài liệu chính thức](https://www.sea-ql.org/SeaORM/): dùng phiên bản tài liệu tương ứng dependency đã khóa.
- [jni-rs — API documentation](https://docs.rs/jni/latest/jni/): link tra cứu hiện hành, **không phải phiên bản đã chốt**; thay bằng URL version cụ thể khi khóa dependency ở Step 3.
- [Java 25 JNI — design/exception handling](https://docs.oracle.com/en/java/javase/25/docs/specs/jni/design.html#exception-handling): quy tắc JNIEnv, reference và pending exception.
- [Baseline dependency Java](../../1pacs-viewer-core/pom.xml): phiên bản dcm4che và native binding đang dùng; Step sử dụng phải ghi revision nguồn khi chốt baseline của tính năng.
