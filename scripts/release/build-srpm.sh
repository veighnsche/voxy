#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -eq 0 ]]; then
  echo "Run this command as a normal user (no sudo)."
  exit 1
fi

if ! command -v git >/dev/null 2>&1; then
  echo "Missing required tool: git"
  exit 1
fi

if ! command -v rpmbuild >/dev/null 2>&1; then
  echo "Missing required tool: rpmbuild"
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

ref="${1:-${VOXY_SOURCE_REF:-HEAD}}"
if ! git rev-parse --verify "${ref}^{commit}" >/dev/null 2>&1; then
  echo "Unknown git ref: ${ref}"
  exit 1
fi

package_name="voxy-app"
spec_src="${repo_root}/packaging/rpm/${package_name}.spec"
if [[ ! -f "${spec_src}" ]]; then
  echo "Missing RPM spec file: ${spec_src}"
  exit 1
fi

upstream_version="$(
  git show "${ref}:Cargo.toml" | awk '
    /^\[workspace\.package\]/ { in_workspace_package = 1; next }
    in_workspace_package && /^\[/ { in_workspace_package = 0 }
    in_workspace_package && $1 == "version" {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  '
)"
if [[ -z "${upstream_version}" ]]; then
  echo "Could not resolve workspace version from ${ref}:Cargo.toml"
  exit 1
fi

to_rpm_version() {
  local version="$1"
  if [[ "${version}" != *-* ]]; then
    printf '%s' "${version}"
    return
  fi

  local base="${version%%-*}"
  local suffix="${version#*-}"
  suffix="${suffix//-/.}"
  printf '%s~%s' "${base}" "${suffix}"
}

rpm_version="$(to_rpm_version "${upstream_version}")"
rpm_release="${VOXY_RPM_RELEASE:-1}"

rpm_topdir="${repo_root}/target/rpm-srpm"
spec_path="${rpm_topdir}/SPECS/${package_name}.spec"
tarball="${rpm_topdir}/SOURCES/${package_name}-${upstream_version}.tar.gz"

echo "Preparing SRPM layout..."
rm -rf "${rpm_topdir}"
mkdir -p \
  "${rpm_topdir}/BUILD" \
  "${rpm_topdir}/BUILDROOT" \
  "${rpm_topdir}/RPMS" \
  "${rpm_topdir}/SOURCES" \
  "${rpm_topdir}/SPECS" \
  "${rpm_topdir}/SRPMS"

echo "Creating source tarball from ref '${ref}'..."
git archive \
  --format=tar \
  --prefix="${package_name}-${upstream_version}/" \
  "${ref}" | gzip -n > "${tarball}"

cp "${spec_src}" "${spec_path}"

echo "Building SRPM (Version=${rpm_version}, Release=${rpm_release})..."
rpmbuild -bs "${spec_path}" \
  --define "_topdir ${rpm_topdir}" \
  --define "voxy_upstream_version ${upstream_version}" \
  --define "pkg_version ${rpm_version}" \
  --define "pkg_release ${rpm_release}" \
  >/dev/null

srpm_path="$(find "${rpm_topdir}/SRPMS" -type f -name "${package_name}-*.src.rpm" | head -n 1)"
if [[ -z "${srpm_path}" ]]; then
  echo "SRPM build finished but package file was not found in ${rpm_topdir}/SRPMS."
  exit 1
fi

echo "SRPM package created: ${srpm_path}"
