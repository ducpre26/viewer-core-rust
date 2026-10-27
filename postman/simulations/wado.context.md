# WADO simulation

Local fixture built 2026-10-27 from `docs/baseline/step-11-contract.md` and Java
fixtures. Owner mock discovery skipped: the CLI is not signed in.

## Purpose

Run the real Rust viewer against the local store in `postman/mocks/wado-store/`.
The viewer itself is never mocked. Payloads and credentials are synthetic.

## Execution

`cargo build --workspace --locked` then `python tools/smoke_wado.py`.
The runner starts/stops Postman and the viewer, materializes the current
application TOML/profile for the configured PostgreSQL, and removes its session.
Local runtime logs are in `target/step-11-logs/` and are not committed.

## Assumptions

- Observed 2026-10-27: runtime smoke passes with configured PostgreSQL, an
  anonymous cache session, and JNI disabled. No new schema or key-image fixture.
- The small mock proves HTTP status/payload/header wiring; slow streams,
  cancellation and timeouts are exercised by Rust TCP tests instead.

## Observations

<!-- begin:observations -->
- 2026-10-27: real viewer smoke passed; the sole `store` member served WADO-RS,
  WADO-URI and error requests. No silent members. Session cleanup and graceful
  application shutdown passed; no external PACS contacted.
<!-- end:observations -->

## What didn't work

- 2026-10-27: CLI 1.75 requires the member path to name `config.yaml`, not the
  mock directory. Fixed in `wado.sim.yaml`.
- 2026-10-27: sandbox prevented CLI dataset cache writes in `~/.postman`.
  The successful run used approved execution with access to that local cache.
