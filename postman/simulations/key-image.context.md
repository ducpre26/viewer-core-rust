# Step 13 local STOW dependency

Source: Java revision 2373896f StowRSServiceImpl.stowRs and FileStorageServiceImpl.writeKeyImage.
Assumption: all 2xx are success even when body reports failed instances; metadata.json is sent with DICOM files.
Verification: tools/smoke_key_image.py runs real Rust/JNI/PostgreSQL against this simulation; results in target/step-13-logs.
This mock uses synthetic identifiers/auth only. No real PACS writes or remote publishing. GET series/metadata return an empty remote list to exercise local augmentation. Other routes return 501.
Limit: source contract only; does not certify behavior of a production PACS.

Observed 2026-11-06: PostgreSQL/JNI router integration passed, including 100 actual POSTs for 101 eligible rows (plus retry=4 exclusion), retry boundary and status-202 cleanup. Binary TCP ingest/local WADO and exporting a persisted batch after restart also passed. Request observations confirm forwarded auth, DICOM bytes and metadata.json in the STOW body. No owner search was performed: this is an explicit local synthetic dependency, not a claim about any hosted PACS contract. Installed CLI 1.75.0 supports simulations but rejects `postman status` as unknown; no account or workspace mutation was performed.
