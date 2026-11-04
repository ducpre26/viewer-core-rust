# Viewer Core Rust

[Build image trên Windows/Linux và deploy Docker](deploy/docker/README.md) —
[plan đã chốt](docs/DOCKER_DEPLOYMENT_PLAN.md). Bộ Docker dành cho thử nghiệm;
image/runtime Linux chưa được kiểm chứng trên máy phát triển hiện tại.

[Hướng dẫn cấu trúc repo cho người mới học Rust](docs/REPOSITORY_GUIDE.md).

[Các lệnh trong vòng đời phát triển](docs/DEVELOPMENT_COMMANDS.md) — chạy dev,
kiểm tra, build release và xem thay đổi trước khi commit.

Backend Rust cho quá trình chuyển từ Java, với HTTP, PostgreSQL và JVM/JNI.
Download Step 12 đã có code và bằng chứng kiểm tra; tiến độ cùng các kiểm tra
còn lại được ghi tại [Step 12](docs/step/step-12-dicom-download.md).

## Download và dependency hiện tại

Rust không dùng Orthanc transcode/native, khác Java.

| Output | Xử lý trong repo Rust |
|---|---|
| DICOM | Trả nguyên byte từ store, giữ transfer syntax, kể cả trong ZIP. |
| JPEG | Lấy từng frame qua WADO-RS rendered của store; không render local. |
| ZIP study/series | Rust đóng gói; Java chỉ parse metadata và tạo tên đường dẫn. |
| MP4 | JavaCV/FFmpeg qua JVM/JNI theo T14. |

DICOM/JPEG instance chạy được khi JNI tắt. Store phải hỗ trợ rendered-frame JPEG. Xem [contract download](docs/baseline/step-12-contract.md)
Không cần section `[download]`. Chỉ `[video].fps` tùy chọn, mặc định 1 như Java;
`DOWNLOAD_PATH_FORMAT` vẫn ở bảng `config` và có fallback. File tạm dùng vùng
riêng trong thư mục tạm hệ thống. Khi nâng cấp, bỏ `[download]` cũ và chuyển
`download.video_fps` sang `[video].fps` nếu cần giữ FPS tùy chỉnh.

## Run

Sao chép `config/app-dev.example.toml` thành `config/app-dev.toml`, điền DB/user/password và Basic admin trước khi chạy.

Install Rust through rustup and the native build tools (MSVC C++ Build Tools on
Windows; a C/C++ compiler and linker on Linux x86_64). Run from this directory:

```text
cargo run --locked -p viewer-app
```

The toolchain is pinned in `rust-toolchain.toml`, and all dependency versions are
locked in `Cargo.lock`. Summer 0.7.2, summer-web 0.7.3 and summer-job 0.7.1 are the
initial implementation selection, subject to the checks recorded in Step 1.
Summer-web currently pulls in aide 0.16.0-alpha.4 for OpenAPI; it is locked too.

| Endpoint | Purpose |
|---|---|
| `/ws/rest/health` | Process liveness |
| `/ws/rest/ready` | Checks PostgreSQL and, when enabled, JVM/facade readiness |
| `/ws/rest/docs/openapi.json` | Generated OpenAPI for the infrastructure routes |
| `/ws/rest/v1/admin/client`, `/ws/rest/v1/admin/client/{id}` | Admin client list/create/update/get/delete (Step 4) |
| `/ws/rest/v1/admin/permission`, `/ws/rest/v1/admin/permission/{id}` | Admin permission catalog CRUD (Step 4) |
| `/ws/rest/v1/admin/client/{id}/permission` | Admin client permission list/assign (Step 4) |
| `/ws/rest/v1/admin/client/{id}/permission/{permissionId}` | Admin client permission removal (Step 4) |

Download dùng `/ws/rest/v1/session/{sessionID}/download/studies/{studyUID}` và
các route series/instance bên dưới, theo phạm vi tại Step 12.

Default address: `127.0.0.1:8888`. Stop with Ctrl+C on Windows or SIGTERM/Ctrl+C on
Linux. Summer's web server drains in-flight requests and its job plugin shuts
down the in-memory scheduler. No business jobs are registered.

## Configuration

Dùng cơ chế mặc định của Summer, không fork/patch framework:

- `config/app.toml`: cấu hình nền.
- `config/app-dev.toml`, `app-test.toml`, `app-prod.toml`: overlay theo profile;
  mặc định dev, chọn bằng `SUMMER_ENV` như trước.
- Giữ `.env`/placeholder `${NAME:default}` trong TOML và `RUST_LOG` override.
- DB/auth/session/JNI nhận typed config từ cùng registry với HTTP/logger;
  module không tự đọc `VIEWER_*`/`PACS_*` hoặc gọi `from_env()`.

Local PowerShell, chạy từ root repo:

```powershell
Copy-Item config/app-dev.example.toml config/app-dev.toml
# Edit database.url/username/password and auth.admin_password, then:
cargo run --locked -p viewer-app
```

[File mẫu](config/app-dev.example.toml) dùng literal, cổng 8889; không cần khai
báo từng biến môi trường. File `app-dev.toml` được gitignore để giữ mật khẩu local.
`config/app-prod.toml` vẫn là overlay production hiện có. Không có CLI `--config`
mới; giữ thư mục config cạnh working directory khi deploy.

Schema/default/validation và phạm vi kiểm thử tại
[contract TOML](docs/SPRING_TO_RUST_JVM_JNI_TECHNICAL_REFERENCE.md#toml-config).

## Boundaries

`crates/viewer-app/src/api` owns HTTP infrastructure. `domain` is reserved for
business rules, `persistence` for repository adapters, `auth` for session and
permission checks, and `dicom` for the DICOM/JNI boundary. Empty modules declare
ownership only; they do not claim any migrated capability.

## Create viewer session (Step 8)

`POST /ws/rest/client/link` accepts `storeUrl`, `storeAuth`, `studies`, `user`
and `expireIn` (minutes, a 32-bit integer or integer string), and returns `{link, session}`. Basic Auth and IP policy
use Step 3a. The deprecated `POST /ws/rest/client/getlink` accepts `studyUIDs`
(study-to-store map), `userID` and `expireIn`, and returns the Java URL envelope.
Validation follows Java, including the legacy nested-store exceptions.

New sessions use the shared short-term cache only; no SESSION row is inserted.
Permission IDs are read from the client's current mappings when creating the
session. With client security disabled, permissions remain null internally and
are omitted from the JSON response. All null session fields are omitted;
empty arrays, empty strings and false booleans remain present. Basic sysadmin
must also have a CLIENT row to create a session (otherwise 404), and receives
only its explicitly assigned permission IDs.

| TOML key | Default | Purpose |
|---|---|---|
| `session.default_expire_minutes` | `60` | Default and maximum requested lifetime; missing/non-positive/over-limit `expireIn` uses this value. |
| `session.path_prefix` | `/viewer?session=` | Viewer link prefix when CONFIG `PATH_PREFIX` is absent. |
| `session.public_domain` | empty string | Public sharing domain when CONFIG `PUBLIC_DOMAIN` is absent. |
| `session.maximum_share_minutes` | `129600` | Maximum shared lifetime; non-positive/over-limit requests fall back to 60 minutes. |

SessionConfig nhận các khóa này từ registry TOML của Summer.

CONFIG `PATH_PREFIX` takes priority and shares the Step 5 cache/invalidation.
Store credentials are included in `/link`'s session response as in Java; they
are not logged. WADO metadata prefetch is deferred for a later Step 8 follow-up
by the scope decision on 2026-10-27; it is excluded from Step 11.
Source comparison and verification: [Step 8](docs/step/step-08-create-session.md).

WADO-RS study/series/instance/frame/thumbnail/metadata/bulk-data retrieval and
WADO-URI are available in [Step 11](docs/step/step-11-wado-retrieval.md).
Payloads stream directly from the session's store, retaining upstream status
and media type. Study scope and session/share-key authentication follow Java.
Local key-image overlays and metadata prefetch are excluded from this step.

Step 8/9 accept `expireIn: 1` and `expireIn: "1"` identically, including
trimmed/signed integer strings. Missing/null/blank strings use the existing
default for session creation; sharing still requires a non-null duration.
Invalid strings, unsupported types and values outside the signed 32-bit range
are rejected before creating a session. The same parser covers legacy getlink.

JSON/body errors in these routes retain HTTP 500 and the envelope
`{status, errorCode: "INTERNAL_ERROR", message, timestamp}` for compatibility.
Messages identify the expected type of a known field, a JSON syntax location,
a required JSON object, or a missing JSON Content-Type. They do not echo input
values, study IDs or internal exception text. Message text is Rust-specific;
byte-for-byte equality with Spring/Jackson exception messages is not required.
Missing required DTO fields still use HTTP 400 / `VALIDATION_ERROR`.

## Authentication/session foundation (Step 3a)

`auth::secure` wraps feature routes with the Java-compatible request filter,
CORS and response headers. Features attach `require_admin` or
`require_permission` as route layers. The request owns its `SecurityContext`;
session access does not grant a Basic admin role. No business or fixture-seeding
endpoint is added by this step.

| TOML key | Contract |
|---|---|
| `auth.client_security_disabled` | Default false; bypass client Basic/permission only; session validation and admin Basic remain active. |
| `auth.admin_username`, `auth.admin_password` | Strings from effective TOML; defaults remain sysadmin as before; set explicit credentials for deployment. |

AuthConfig nhận TOML; middleware business rules giữ nguyên Step 3a.

Startup adds `client` and `session` via transactional migrations, serialized
with a PostgreSQL advisory lock. Auth caches clients for 600 minutes (10 entries)
and session snapshots for 120 minutes (1000 entries); business expiry is checked
separately. `Auth::invalidate_client` and `invalidate_session` support later CRUD
steps. Session permission snapshots are preserved until invalidation/expiry.

Source comparison and test scope: [Step 3a](docs/step/step-03a-auth-session-foundation.md).
The documented PostgreSQL test command includes the opt-in auth HTTP suite and
real TCP peer checks. Java is read as source only; no Java baseline tests are run.

## Verify

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
python tools/smoke_app.py --release
```

The smoke test uses real child processes and ephemeral localhost ports. It checks
dev/prod configuration, health/readiness, generated OpenAPI, unknown routes,
scheduler startup, production JSON logging, and clean signal-driven shutdown.
On Windows it creates a hidden private console to send Ctrl+C safely. The GitHub
Actions workflow runs the same checks on Windows and Linux x86_64 when this
project is hosted in its own repository. Java 25 is required when JNI is enabled.
PostgreSQL is mandatory (see below).

Version-matched upstream references:
[summer](https://docs.rs/summer/0.7.2/summer/),
[summer-web](https://docs.rs/summer-web/0.7.3/summer_web/),
[summer-job](https://docs.rs/summer-job/0.7.1/summer_job/).

Track actual completion and remaining validation in
[Step 1](docs/step/step-01-rust-application.md).

## Persistence (Step 02)

PostgreSQL is mandatory. Startup connects, runs `SELECT 1`, then applies all
pending SeaORM migrations before HTTP starts. Failure exits with status 1 and a
sanitized error. `/health` remains process liveness; `/ready` queries PostgreSQL
and returns 200 with `postgresql:up`, or 503 with `postgresql:down`.

`[database]` trong TOML chứa `url`, `username`, `password`,
`max_connections` (default 10, >0) và `timeout_seconds` (default 5, >0).
URL không nhúng credential; code encode hai trường credential riêng và giữ nguyên
password. Cấu hình được validate trước khi mở pool; driver SQL logging vẫn tắt.
Không in config/driver text/URL/credential vào lỗi.

Migration registration is `persistence::Migrator::migrations()`. Step 3a adds
`client` and `session` alongside SeaORM's `seaql_migrations` tracking table. Step 4 adds
`permission` and `client_permission`, unique/composite keys and cascading foreign keys.
Each later step
adds its own entity mapping, migration, repository and tests. Do not edit applied
migrations; register a new migration to evolve the schema. Do not import Java's
whole schema. Startup runs pending migrations automatically and requires DDL
permission. Concurrent migration startup is serialized by a transaction-scoped
advisory lock.

Repositories live under `persistence`, borrow `&impl ConnectionTrait`, and return
`Result<_, DbErr>`. Application services own transaction boundaries using
`TransactionTrait::begin()`, pass `&DatabaseTransaction` to participating
repositories, then explicitly `commit()` or `rollback()`. Avoid nested transactions
inside repositories and external network calls while holding a transaction.
HTTP handlers must map errors without exposing driver text. The shared pool is
closed explicitly after HTTP drains and scheduler shutdown completes.

Run the real PostgreSQL integration test in addition to the standard checks:

```text
cargo test -p viewer-app --locked postgres_migrations_transaction_readiness_and_close -- --ignored
python tools/smoke_app.py
```

CI provisions a dedicated PostgreSQL 18 instance on each Windows/Linux runner.
Local PostgreSQL 18 database `viewer_rust` was created separately; no business
tables are needed in Step 02.

## JVM/JNI (Step 03)

JNI defaults to disabled. Dùng `jni.enabled=true` trong TOML và
các khóa runtime/artifact/limit in [the adapter guide](crates/dicom-jvm/README.md).
Build the Java 25 [facade](java/dicom-facade/README.md) first. It advertises API 2
with downloadMetadata/downloadSeries/encodeVideo and an empty codec list.
API 2 removes download quota arguments; deploy the rebuilt facade with the Rust binary.
The adapter pins jni-rs 0.21.1 with invocation,
validates the distribution/checksums, and runs synchronous JNI on a dedicated worker.

Startup checks JVM/facade before HTTP. Readiness adds `jvm:up` or `jvm:down` when
enabled. Normal shutdown releases references, detaches, destroys the JVM and then
closes PostgreSQL. An unrecoverable call or bounded shutdown failure exits the
process; no in-process JVM restart is attempted.

See [development commands](docs/DEVELOPMENT_COMMANDS.md) for isolated JVM tests and
HTTP regression with JNI enabled. Logs are in `target/step-03-logs/`. Runtime
production distribution and P1–P4 limits remain open; test limits are not approval
of production sizing. Completion evidence is recorded in
[Step 3](docs/step/step-03-jvm-jni-foundation.md).
