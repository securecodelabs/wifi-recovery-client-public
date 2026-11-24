#!/usr/bin/env bash
set -euo pipefail
mkdir -p .tools/security ci-reports
install_tool() {
  local name="$1" url="$2" checksum="$3" member="$4"
  local archive=".tools/security/$name.tar.gz"
  curl --fail --location --silent --show-error "$url" --output "$archive"
  printf '%s  %s\n' "$checksum" "$archive" | sha256sum --check --status
  tar -xzf "$archive" -C .tools/security "$member"
  if [[ "$member" != "$name" ]]; then cp ".tools/security/$member" ".tools/security/$name"; fi
  chmod +x ".tools/security/$name"
}
install_tool actionlint https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_linux_amd64.tar.gz 8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8 actionlint
install_tool gitleaks https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_linux_x64.tar.gz 551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb gitleaks
install_tool cargo-audit https://github.com/rustsec/rustsec/releases/download/cargo-audit/v0.22.2/cargo-audit-x86_64-unknown-linux-musl-v0.22.2.tgz 7fb9497f8594b389e5fce5ef9b92db08432996895b2e0c5a0167a69ed445c428 cargo-audit-x86_64-unknown-linux-musl-v0.22.2/cargo-audit
install_tool cargo-deny https://github.com/EmbarkStudios/cargo-deny/releases/download/0.22.0/cargo-deny-0.22.0-x86_64-unknown-linux-musl.tar.gz 98c56e6a408bfa5c3c60744b6015aaa6aa08f743fdeb88386d06fca035bb1786 cargo-deny-0.22.0-x86_64-unknown-linux-musl/cargo-deny
python3 -m venv .tools/security/venv
.tools/security/venv/bin/pip install --disable-pip-version-check zizmor==1.32.0
.tools/security/actionlint
.tools/security/venv/bin/zizmor --offline --min-severity low .github/workflows
.tools/security/cargo-audit audit --deny warnings --json > ci-reports/audit.json
.tools/security/cargo-deny --locked check licenses sources
.tools/security/gitleaks dir --redact --no-banner --report-format json --report-path ci-reports/secrets-files.json .
if git rev-parse --verify HEAD >/dev/null 2>&1; then
  .tools/security/gitleaks git --redact --no-banner --log-opts=--all --report-format json --report-path ci-reports/secrets-history.json .
fi
pwsh -NoProfile -File tools/licenses.ps1 -Check
