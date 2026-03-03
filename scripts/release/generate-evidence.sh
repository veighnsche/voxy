#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <version|vX.Y.Z>" >&2
  exit 1
fi

version="${1#v}"
tag="v${version}"
output_dir="${repo_root}/docs/release-evidence"
output_path="${output_dir}/${tag}.md"

release_commit="$(git rev-parse HEAD)"
if git rev-parse --verify "${tag}^{commit}" >/dev/null 2>&1; then
  release_commit="$(git rev-list -n 1 "${tag}")"
fi

last_known_good_tag="$(
  git tag --list 'v*' \
    | grep -Fxv "${tag}" \
    | sort -V \
    | tail -n 1 \
    || true
)"

ci_checks_url=""
security_scan_url=""
release_workflow_url=""

if command -v gh >/dev/null 2>&1; then
  ci_checks_url="$(
    gh run list \
      --workflow "CI" \
      --commit "${release_commit}" \
      --limit 20 \
      --json url,conclusion \
      --jq 'map(select(.conclusion=="success"))[0].url // ""' \
      2>/dev/null || true
  )"
  security_scan_url="$(
    gh run list \
      --workflow "CI" \
      --commit "${release_commit}" \
      --limit 20 \
      --json url,conclusion \
      --jq 'map(select(.conclusion=="success"))[0].url // ""' \
      2>/dev/null || true
  )"
  release_workflow_url="$(
    gh run list \
      --workflow "Release" \
      --commit "${release_commit}" \
      --limit 20 \
      --json url,conclusion \
      --jq 'map(select(.conclusion=="success"))[0].url // ""' \
      2>/dev/null || true
  )"
fi

mkdir -p "${output_dir}"

cat > "${output_path}" <<EOF
# Release Evidence: ${tag}

Generated at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")

## Release Identity

- Release tag: ${tag}
- Release commit: ${release_commit}
- Release owner:
- 48h on-call owner:
- Date/time (UTC):

## Required Gate Evidence

- CI checks URL: ${ci_checks_url}
- Security scan URL: ${security_scan_url}
- Release workflow URL: ${release_workflow_url}
- Packaging validation evidence:
  - \`VOXY_PACKAGING_VALIDATE_STRICT=1 ./scripts/release/validate-packaging.sh\`

## Runtime Validation

- Fixture e2e output:
- Live e2e output (if run):
- Manual QA notes (mic + backend):

## Artifact Integrity

- Artifact list:
  - \`voxy-app-${version}-linux-x86_64.tar.gz\`
  - \`screenshots/idle.png\`
  - \`screenshots/recording.png\`
  - \`screenshots/config.png\`
- SHA256SUMS reference:
- Local checksum verification result:

## Rollback Readiness

- Last known good tag: ${last_known_good_tag}
- Rollback rehearsal date:
- Rollback rehearsal operator:
- Rollback rehearsal notes:

## Decision

- Known issues:
- Go / No-Go:
- Approver:
EOF

echo "[release] Wrote evidence template: ${output_path}"
