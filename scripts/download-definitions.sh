#!/usr/bin/env bash
# download-definitions.sh — Fetch and verify FHIR R5 definition artifacts.
#
# Usage: scripts/download-definitions.sh [--force]
#   --force   Re-download even if files already exist.
#
# Files are placed in definitions/r5/. SHA-256 checksums are verified before
# unpacking. The downloaded archives are removed after extraction.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUT_DIR="${ROOT_DIR}/definitions/r5"

FORCE=false
for arg in "$@"; do
    if [[ "${arg}" == "--force" ]]; then
        FORCE=true
    fi
done

# FHIR R5 release artifacts
DEFINITIONS_URL="https://hl7.org/fhir/R5/definitions.json.zip"
EXAMPLES_URL="https://hl7.org/fhir/R5/examples-json.zip"

# SHA-256 checksums (update when a new FHIR R5 patch is released)
DEFINITIONS_SHA256="b97218f6e13b62a4b7c6de5b1a83a39f2c34bfad37d3de10d498b4e07f218e2f"
EXAMPLES_SHA256="e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

# ---------------------------------------------------------------------------

require_cmd() {
    if ! command -v "$1" &>/dev/null; then
        echo "ERROR: '$1' is required but not found in PATH." >&2
        exit 2
    fi
}

require_cmd curl
require_cmd sha256sum
require_cmd unzip

mkdir -p "${OUT_DIR}"

download_and_verify() {
    local url="$1"
    local expected_sha256="$2"
    local archive
    archive="${OUT_DIR}/$(basename "${url}")"
    local sentinel="${archive%.zip}.sentinel"

    if [[ "${FORCE}" == false && -f "${sentinel}" ]]; then
        echo "[skip] $(basename "${archive}") already downloaded and verified."
        return
    fi

    echo "[fetch] ${url}"
    curl --fail --silent --show-error --location --output "${archive}" "${url}"

    echo "[verify] $(basename "${archive}")"
    local actual_sha256
    actual_sha256="$(sha256sum "${archive}" | awk '{print $1}')"
    if [[ "${actual_sha256}" != "${expected_sha256}" ]]; then
        echo "ERROR: SHA-256 mismatch for $(basename "${archive}")." >&2
        echo "  expected: ${expected_sha256}" >&2
        echo "  actual:   ${actual_sha256}" >&2
        rm -f "${archive}"
        exit 1
    fi

    echo "[unzip] $(basename "${archive}")"
    unzip -q -o "${archive}" -d "${OUT_DIR}"
    rm -f "${archive}"
    touch "${sentinel}"
    echo "[ok] $(basename "${archive}") extracted to ${OUT_DIR}"
}

download_and_verify "${DEFINITIONS_URL}" "${DEFINITIONS_SHA256}"
download_and_verify "${EXAMPLES_URL}"    "${EXAMPLES_SHA256}"

echo ""
echo "FHIR R5 definitions ready in ${OUT_DIR}"
