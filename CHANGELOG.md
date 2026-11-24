# Changelog

## 0.1.1 — 2026-11-24

- Require complete job responses and reject a UUID that differs from the requested job.
- Enforce 0-86400-second runtime limits and regular input files of 1 byte to 8 MiB before submission.
- Preserve HTTP status and useful messages for non-JSON error responses.
- Validate resource envelopes and result counts before display/export.
- Return normal error exits for malformed responses and closed output pipes, including license output.
- Run HTTP integration tests against the exact packaged executable in CI and releases; add protocol, boundary, output and regression tests.

The CLI continues to support HTTP contract v1 on Linux and Windows amd64. Releases select one platform's executable plus its SHA-256 checksum; this release selects Linux.

## 0.1.0 — 2026-11-24

Initial public release: submission, job control, resource/progress monitoring, result export, JSON/NDJSON and documented exit codes.
