# Java baseline — collected reference

This evidence is retained for reuse. It is not an entry gate for Rust initialization. Capture runtime fixtures and schema details only for the feature being implemented; perform whole-system reconciliation at Step 18. The remaining items below are not prerequisites for Step 1.

Source branch: `codex/java-baseline-step-00`, created from `refactor` at
`c983e4eac389fa6b31f970cfee8f9fce5e20af4c` on 2026-10-22.
No Java source, pom or database schema changes were pending at capture.
Existing local agent configuration and documentation changes were retained.
No commit or push was performed. A branch is mutable: use the commit hash as the baseline identity.

## Captured evidence

- [Revision](source-revision.json) and [SHA-256 manifest](input-hashes.json).
- `inputs/`: 14 use cases, entity model, use case diagram and schema snapshot. UC/model documents were untracked in Java; copies preserve the exact input independently of the branch.
- [Source inventory](endpoint-inventory.md): 75 unique HTTP method/path pairs, including 2 excluded seal routes. All 14 UC IDs represented. This is static extraction, not Spring runtime evidence or a complete traceability review.
- [UC checklist](uc-checklist.md): assertions extracted from specifications; concrete inputs and observed expected results remain pending.
- [Configuration names](configuration-keys.md): no credential values copied; contract review remains pending.

Regenerate static extraction from the Rust project root using `python tools/baseline_inventory.py`.
This overwrites snapshots; do not regenerate against a changed Java revision when preserving this baseline.

## Verification on 2026-10-22

- Java: Temurin 25.0.4.1+1, Windows; experimental build environment, not a production decision.
- `mvn -Dmaven.repo.local=C:/Users/Admin/.m2/repository -DskipTests compile`: BUILD SUCCESS, 262 source files. Tests were not run.
- Offline compile initially failed because Maven plugin dependencies were unavailable; online compile resolved them.
- `GET http://localhost:8888/v3/api-docs`: connection refused. No runtime OpenAPI was captured.
- Static inventory validation: 75/75 unique method/path pairs; UC-001 through UC-014 plus explicit seal exclusions.

## Remaining work — per feature and final coverage review

1. Prepare an isolated Java runtime with synthetic database and DICOM/service dependencies; record startup configuration. Existing target JAR has an older version and must not be treated as this revision.
2. Export Spring mappings/OpenAPI and reconcile HTTP, framework and WebSocket surfaces against source inventory.
3. Review service/repository/entity and WebRequestFilter authorization for every route; static service receivers do not prove transitive effects.
4. Complete configuration/schema contract, including actual entity/schema differences. The copied SQL is a source snapshot, not an approved migration or live database dump.
5. Capture synthetic request/response/DB effects, golden DICOM corpus, print/error scenarios and benchmark with recorded workload and environment.
6. Attach concrete expected results and fixture IDs to every UC checklist item; verify PHI/credential exclusion before completion.

P1–P4 remain open and do not block source inventory. No performance result, runtime fixture, approval or completion is claimed.
