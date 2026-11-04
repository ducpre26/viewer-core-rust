# Download failure simulation

Created 2026-10-28 from the Step 12 download contract. Synthetic local data only;
owner mock discovery skipped because the CLI is not signed in.

## Purpose

Run the real Rust backend against `postman/mocks/download-store/`, with real
JavaCV/FFmpeg for video. Controlled HTTP faults never touch the configured PACS.

## Execution

Build the debug backend/facade, then run `python tools/test_download_failures.py
--java-home <JDK25>`. The runner creates synthetic fixtures and starts/stops the
simulation and backend. Runtime evidence stays under `target/step-12-error-logs/`.

## Assumptions

Only the documented download dependency paths are modeled. The local control
endpoint selects faults; it is not part of the real store API. This run is not
a full-disk, performance/soak or platform-certification test.

## Observations

<!-- begin:observations -->
2026-10-28: real backend exercised against the synthetic local store. Store
401/403/404/429/5xx, truncated bodies, metadata/second-instance/second-frame
failures produce sanitized 500 before download headers; stalls produce 504.
Gated source confirms no downstream headers before upstream EOF. Real JavaCV
encoded 384 noisy 512x512 frames, independently decoded to the full frame count.
Disconnect and deadline while real MP4 packets were being written retain the
native lease/permit until completion; TTL skips the active job. Readiness recovers.
Store failure after committed headers does not affect the staged download;
SHA-256 matches source. Disconnect during response releases resources. Artificial
staged-file truncation is detected by the HTTP client through Content-Length.
Evidence: target/step-12-error-logs/results.json. No production store was used.
<!-- end:observations -->

## What didn't work

Refactor 2026-11-03: the runner no longer supplies `[download]`, download admission
quotas or transport deadlines. Each JNI call uses the shared `jni.timeout_ms`.
DICOM transport remains available during native encoding/overdue JNI work.
System TEMP is isolated per child process; workspace tests cover orphan locks.
The 2026-10-28 observations above are historical, not results for this revision.

Current evidence: 26 selected checks passed on 2026-11-03 in
`target/step-12-error-logs/results.json`, including slow headers/body returning
200, upstream errors, real FFmpeg cancellation/deadline, DICOM transport staying
available during encode/overdue JNI, and file cleanup after response disconnect.
This does not establish full-disk, memory/soak, broad codec or Linux behavior.

- First run exposed DICOM 401 being returned as a successful 200 attachment with
  the store JSON error. Production now requires source HTTP 200 for every fetch.
- A simulation gateway turns an abruptly closed member body into a stalled
  response. Wire-level faults use that member's direct listener; the gated
  staging check also runs through the simulation gateway.
- localhost resolved through a slow IPv6 fallback on this host; the runner uses
  127.0.0.1. Initial harness failures are not counted as successful checks.
- The runner does not establish full-disk, memory/soak, broad codec corpus,
  release/Linux or production resource settings; those remain deferred.
