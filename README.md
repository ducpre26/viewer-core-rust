# Viewer Core Rust

[Hướng dẫn cấu trúc repo cho người mới học Rust](docs/REPOSITORY_GUIDE.md).

[Các lệnh trong vòng đời phát triển](docs/DEVELOPMENT_COMMANDS.md) — chạy dev,
kiểm tra, build release và xem thay đổi trước khi commit.

Infrastructure for the Java-to-Rust migration, including PostgreSQL persistence.
JVM/JNI foundation is implemented; business DICOM operations are not implemented yet.

## Run

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

Default address: `127.0.0.1:8888`. Stop with Ctrl+C on Windows or SIGTERM/Ctrl+C on
Linux. Summer's web server drains in-flight requests and its job plugin shuts
down the in-memory scheduler. No business jobs are registered.

## Configuration

`config/app.toml` is the base configuration. `SUMMER_ENV=prod` additionally loads
`config/app-prod.toml`, enabling JSON logs and binding to `0.0.0.0` by default.
Use `SUMMER_ENV=dev` for local development. Environment overrides:

| Variable | Default | Purpose |
|---|---|---|
| `VIEWER_BIND` | `127.0.0.1` in dev | HTTP bind address |
| `VIEWER_PORT` | `8888` | HTTP port; choose another port if Java is running |
| `VIEWER_LOG_LEVEL` | `info` | Tracing level |
| `VIEWER_CONFIG` | `config/app.toml` | Alternative base configuration file |

Summer also honors `RUST_LOG`; when set, it takes precedence over
`VIEWER_LOG_LEVEL`. The smoke test clears inherited `RUST_LOG` so it can verify
the configured log level.

Example PowerShell: `$env:VIEWER_PORT='8889'; cargo run --locked -p viewer-app`.
Example Linux: `SUMMER_ENV=prod VIEWER_PORT=8889 ./target/release/viewer-app`.
Keep `config/` alongside the working directory when deploying. Summer reads the
default configuration during builder creation even with a custom config path.
Request URI/header/body logging is deliberately not enabled, to avoid recording
session query parameters or medical data as business routes are introduced.

## Boundaries

`crates/viewer-app/src/api` owns HTTP infrastructure. `domain` is reserved for
business rules, `persistence` for repository adapters, `auth` for session and
permission checks, and `dicom` for the DICOM/JNI boundary. Empty modules declare
ownership only; they do not claim any migrated capability.

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

Set these environment variables before running any app/smoke/integration command:

```powershell
$env:VIEWER_DATABASE_URL='postgresql://127.0.0.1:5432/viewer_rust'
$env:VIEWER_DATABASE_USER='postgres'
# Supply VIEWER_DATABASE_PASSWORD through your local environment/secret manager.
cargo run --locked -p viewer-app
```

`VIEWER_DATABASE_URL` is required. Credentials may be included in the URL
(percent-encode reserved characters), or overridden by `VIEWER_DATABASE_USER`
and `VIEWER_DATABASE_PASSWORD`. No credential is saved in the repository.
`VIEWER_DATABASE_MAX_CONNECTIONS` defaults to 10; the minimum is 1.
`VIEWER_DATABASE_TIMEOUT_SECONDS` defaults to 5 for connect/acquire/readiness.
Both settings must be positive integers. SQL driver statement logging is disabled;
driver error text and connection URLs are never printed by this adapter.

Migration registration is `persistence::Migrator::migrations()`. It is currently
empty; only SeaORM's `seaql_migrations` tracking table is created. Each later step
adds its own entity mapping, migration, repository and tests. Do not edit applied
migrations; register a new migration to evolve the schema. Do not import Java's
whole schema. Startup runs pending migrations automatically and requires DDL
permission. Run only one application instance during schema deployment; concurrent
migration orchestration is outside this step.

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

JNI defaults to disabled. Enable with `VIEWER_JNI_ENABLED=true` and the explicit
runtime, artifact and limit settings in [the adapter guide](crates/dicom-jvm/README.md).
Build the Java 25 [facade](java/dicom-facade/README.md) first. It advertises API 1
and empty operation/codec lists. The adapter pins jni-rs 0.21.1 with invocation,
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
