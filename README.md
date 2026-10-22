# Viewer Core Rust

Step 1 infrastructure bootstrap for the Java-to-Rust migration. Business modules,
PostgreSQL and JVM/JNI are not implemented yet.

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
| `/ws/rest/ready` | Ready for the implemented infrastructure; no external dependencies yet |
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
project is hosted in its own repository. No Java runtime or database is required.

Version-matched upstream references:
[summer](https://docs.rs/summer/0.7.2/summer/),
[summer-web](https://docs.rs/summer-web/0.7.3/summer_web/),
[summer-job](https://docs.rs/summer-job/0.7.1/summer_job/).

Track actual completion and remaining validation in
[Step 1](docs/step/step-01-rust-application.md).
