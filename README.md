# WiFi recovery CLI

A Rust command-line client for HTTP contract v1 of `wifi-recovery-api`. It submits HC22000 pattern searches, monitors jobs and Linux/AMD resources, stops and resumes work, and exports verified results. There are no login or token commands and no full-screen TUI.

The product is one executable, `wifi-recovery-client` (Windows: `.exe`). There are no Docker files, images, installers or service definitions. Both Windows and Linux amd64 are supported. Download the Linux amd64 executable and checksum from [GitHub Releases](https://github.com/securecodelabs/wifi-recovery-client-public/releases); Windows users can build the same source with Cargo.

## Commands

Set `WIFI_RECOVERY_URL=http://server:8080`, or pass `--server` to any command.

```sh
wifi-recovery-client info
wifi-recovery-client info --watch
wifi-recovery-client pattern preview 'Home?d?d?d?d'
wifi-recovery-client submit input.hc22000 --pattern 'Home?d?d?d?d' --runtime 2h --limit 0
wifi-recovery-client jobs
wifi-recovery-client status JOB_ID
wifi-recovery-client watch JOB_ID
wifi-recovery-client stop JOB_ID
wifi-recovery-client resume JOB_ID
wifi-recovery-client result JOB_ID --output result.json
wifi-recovery-client delete JOB_ID --yes
```

`--json` produces JSON; watches produce one JSON snapshot per line. `--interval` sets polling seconds (0.1-3600, default 2). Full UUIDs are preserved in output. An unambiguous prefix of at least four characters can be used for job commands. Lists fetch every page.

Runtime accepts whole seconds or `s`, `m`, `h` suffixes. Zero runtime/limit means unlimited. `submit --request-id UUID` binds retries to the input and normalized settings; the server rejects different content or reuse after deletion. Submissions are not automatically retried. Stopping is explicit: Ctrl+C exits the local watch without stopping the server job. Failed polling also leaves server work unchanged.

Combined watch output includes pattern, stage, progress, start times in UTC, attempt and total durations, estimates, latest batch candidate, kH/s, CPU, RAM, GPU load and VRAM. kH/s is the engine's run-average PBKDF2 derivation throughput; candidates/s is a separate measurement. Missing/stale values remain unknown. JSON timestamps retain Unix milliseconds. Terminal control characters are escaped.

Results are checked for the requested job identity and verified password structure before display/export. Export creates a new file and refuses to overwrite an existing path. `--version`, `--help` and `--licenses` work without connecting to a server. License notices are embedded so distribution remains executable-only.

| Exit code | Meaning |
| --- | --- |
| 0 | Command completed, or watched job recovered |
| 1 | Connection, API, protocol or output-file error |
| 2 | Invalid command-line syntax |
| 10 | Watched job exhausted its candidates |
| 11 | Watched job stopped |
| 12 | Watched job reached a limit |
| 13 | Watched job failed |
| 14 | Watched job was interrupted |
| 130 | Local watch interrupted with Ctrl+C |

## Build and test

```sh
cargo +1.93.0 fmt --all -- --check
cargo +1.93.0 clippy --locked --all-targets --all-features -- -D warnings
cargo +1.93.0 test --locked --all-targets --all-features
cargo +1.93.0 build --release --locked
```

The Rust harness uses real local HTTP exchanges and the compiled CLI. It covers commands, pagination, prefix ambiguity, JSON/NDJSON, error codes, safe exports and Linux Ctrl+C behavior. See [quality requirements](docs/quality.md) and [measured validation](docs/validation.md). Remote acceptance tests run both native executables under `/home/wifi-service/wifi-suite`; no global installation or container is used.

Releases require an explicit user request. The manual workflow validates an existing source/version tag and publishes one selected platform's executable plus SHA-256 checksum. The first release, v0.1.0, publishes the locally tested Linux amd64 executable. Release notes identify the source revision and published asset checksum. Code is Apache-2.0; [dependency notices](THIRD_PARTY_NOTICES.md) cover third-party components.
