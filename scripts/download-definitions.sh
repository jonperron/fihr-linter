#!/usr/bin/env bash
# download-definitions.sh — Fetch and verify FHIR R5 definition artifacts.
#
# Usage: scripts/download-definitions.sh [--force]
#   --force   Re-download even if files already exist.
#
# Files are placed in definitions/r5/. SHA-256 checksums are verified before
# unpacking. The downloaded archives are removed after extraction.
#
# SECURITY NOTES:
#   - TLS 1.2+ is enforced via --tlsv1.2.
#   - SHA-256 is verified before extraction (supply-chain integrity).
#   - Zip Slip (CWE-22) is mitigated: extracted paths are checked to stay
#     within OUT_DIR before any file is written.
#   - The EXAMPLES_SHA256 below must be set to the real checksum of the
#     FHIR R5 examples-json.zip artifact before enabling that download.
#     Do NOT use the SHA-256 of an empty string as a placeholder.

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

# SHA-256 checksums (update when a new FHIR R5 patch is released).
# Obtain the correct value by running:
#   curl -fsSL <URL> | sha256sum
DEFINITIONS_SHA256="b97218f6e13b62a4b7c6de5b1a83a39f2c34bfad37d3de10d498b4e07f218e2f"
# TODO: replace with the real SHA-256 of examples-json.zip before enabling.
# The placeholder below is intentionally wrong to prevent accidental use.
EXAMPLES_SHA256="0000000000000000000000000000000000000000000000000000000000000000"

# SHA-256 of an empty string — never a valid archive checksum.
# Guard against accidentally committing this value.
EMPTY_SHA256="e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

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
require_cmd python3

mkdir -p "${OUT_DIR}"

# Abort early if any checksum is the SHA-256 of the empty string (placeholder).
for checksum in "${DEFINITIONS_SHA256}" "${EXAMPLES_SHA256}"; do
    if [[ "${checksum}" == "${EMPTY_SHA256}" ]]; then
        echo "ERROR: A SHA-256 checksum is set to the hash of an empty string." >&2
        echo "       Update the checksum constants with the real artifact hashes." >&2
        exit 3
    fi
done

# Verify that all paths extracted from a zip archive stay within OUT_DIR.
# Mitigates Zip Slip (CWE-22).
check_zip_paths() {
    local archive="$1"
    local dest="$2"
    local real_dest
    real_dest="$(realpath "${dest}")"

    while IFS= read -r entry; do
        local target
        target="$(realpath -m "${real_dest}/${entry}")"
        if [[ "${target}" != "${real_dest}"* ]]; then
            echo "ERROR: Zip Slip detected — entry escapes target directory: ${entry}" >&2
            rm -f "${archive}"
            exit 4
        fi
    done < <(python3 -c "
import zipfile, sys
with zipfile.ZipFile(sys.argv[1]) as z:
    for name in z.namelist():
        print(name)
" "${archive}")
}

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
    # --tlsv1.2   : enforce TLS 1.2 minimum (OWASP A02 — Cryptographic Failures)
    # --location  : follow redirects (hl7.org uses HTTP→HTTPS redirect)
    # --fail      : non-200 responses abort with exit code 22
    curl --fail --silent --show-error --location --tlsv1.2 --output "${archive}" "${url}"

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

    echo "[check-paths] $(basename "${archive}")"
    check_zip_paths "${archive}" "${OUT_DIR}"

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
