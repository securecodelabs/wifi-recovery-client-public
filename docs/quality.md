# Local quality gates

Rust 1.93.0 is pinned in rust-toolchain.toml. Run formatting, Clippy across all targets/features with warnings denied, tests and a locked release build. Cargo.toml explicitly denies cognitive complexity above 25, functions above 100 Clippy-counted lines, nesting above 4 and functions with more than 7 arguments. clippy.toml defines the thresholds. Refactor failures; do not increase thresholds or add suppressions.

These lints use `deny`, including explicit `-D` flags in CI. `forbid` cannot be used with Clap's externally generated implementations: its derive macro internally allows lint groups, which conflicts with a command-line forbid. Handwritten project code has no complexity allowances and remains checked with the same limits. Unsafe code is forbidden.

Linux runs `bash .github/security.sh` locally and in CI. It installs checksum-pinned actionlint 1.7.12, Gitleaks 8.30.1, cargo-audit 0.22.2 and cargo-deny 0.22.0 under .tools, and zizmor 1.32.0 in a private tooling environment. It checks workflows, current Rust advisories, allowed dependency licenses/sources, redacted working-tree and full-history secrets, and generated notices. On Windows run the matching tool versions and `tools/licenses.ps1 -Check`; tools have no role in runtime distribution.

`tools/licenses.ps1` regenerates embedded notices from locked Cargo metadata and dependency license files. Its `-Check` mode fails on stale notices. The script includes build/test dependency notices as a conservative superset. Standard license texts and public synthetic fixtures retain their origins; application code, tests and workflows were independently written. Public pcap CI informed the checking design, not a copied application implementation.

CI uses hosted Linux, and the CLI additionally uses hosted Windows. Workflows pin action SHAs, use minimal permissions, bound job time and retain redacted reports for seven days. Weekly checks refresh advisory results; Dependabot groups ordinary dependency changes. CPU engine integration tests do not imply GPU coverage.

Only an explicit release request authorizes tags, repository publication or the manual release workflow. Ordinary builds, tests and commits are local operations. Release assets are one executable and its checksum, with source revision in release notes. Docker and deployment assets are outside scope.
