# Local validation — 2026-11-24

Implementation source: `9b4db82c20b21d88e958903e9a58e82178f51d72`. Subsequent documentation commits do not change the executable. Version: 0.1.0; Rust: 1.93.0. This report records local validation before repository publication. The v0.1.0 release uses the Linux executable identified below; release notes identify the final source revision and hosted CI run.

## Exact executables

The initially packaged Windows executable is `dist/wifi-recovery-client.exe`, built with `cargo +1.93.0 build --release --locked`. Its SHA-256 is:

```text
44ab0c550b8947a77f087738b6db56d4d72045126a957df7ffe7c0944836eac3
```

A native Linux build was also used for compatibility testing. Its SHA-256 is:

```text
a1011345ecc1b361005b4630d9800cedd44ca658946bfdc73e0c154ebccee1fe
```

The Linux test executable lives under `/home/wifi-service/wifi-suite/bin` on the test host. The same binary is selected for the first public release. Local artifacts under `dist` are excluded from Git. Each project builds one program, with no runtime bundle, container, installer or service definition.

## Results

Formatting, all-target/all-feature Clippy, locked tests and release builds passed on Windows and Linux. Clippy denies warnings and enforces cognitive complexity 25, function length 100, nesting depth four and seven arguments. No handwritten complexity suppression or raised threshold was added.

Six tests passed on Windows and seven on Linux, including real HTTP exchanges for every command, pagination, UUID prefix handling, JSON/NDJSON, watch outcomes, invalid responses, verified result export, overwrite refusal and typed contract validation. The additional Linux case confirms Ctrl+C exits a watch without sending a stop request. Both projects round-trip the same versioned job fixture.

The exact Windows release executable was exercised against the real remote API and Radeon engine: resource information, pattern preview, upload/submission, listing, status by short ID, watch, stop, resume, result export and deletion. A synthetic capture recovered 12 verified records; a resumed limit test retained its UUID and produced two attempts; watch reported exit 12 for the limited outcome. The exported result was verified and the two temporary acceptance jobs were deleted. The final Linux build passed native version, info, typed status and embedded-license checks against the final API executable.

actionlint 1.7.12, zizmor 1.32.0, cargo-audit 0.22.2, cargo-deny 0.22.0 and Gitleaks 8.30.1 passed locally, as did generated-license notice verification. The dependency audit covered 196 locked packages with no advisories reported on this date. Gitleaks checked the working tree and local Git history. Hosted CI was pending at the time of this local validation; consult the release notes for its subsequent result.

The companion API's `docs/validation.md` records the exact server executable and Radeon performance gate. Two full API-submitted searches averaged 660.566 and 659.369 kH/s; those measurements apply to the documented single-record workload, not every capture shape. All remote testing files remained under the WiFi Suite home directory; no global application installation was made.
