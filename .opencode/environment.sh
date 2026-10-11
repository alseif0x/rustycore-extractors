# SPDX-License-Identifier: GPL-3.0-or-later
# Source this from the launcher/check helpers; build/output paths stay external.
RCE_OPENCODE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
RCE_OPENCODE_WORK="$(dirname -- "$RCE_OPENCODE_ROOT")/rustycore-extractor-work"
export PATH="${HOME}/.cargo/bin:${PATH}"
export CARGO_TARGET_DIR="$RCE_OPENCODE_WORK/builds/cargo-opencode"
# The neighboring laboratory is inside the enclosing wow Git repository.
# Batch fixtures must use a temporary root outside every Git checkout.
export TMPDIR="${HOME}/.local/share/rustycore-extractor-work/tmp/opencode"
export PYTHONPYCACHEPREFIX="$RCE_OPENCODE_WORK/cache/python-opencode"
mkdir -p -- "$CARGO_TARGET_DIR" "$TMPDIR" "$PYTHONPYCACHEPREFIX" "$RCE_OPENCODE_WORK/reports/opencode"
# Default checks are synthetic; inherited live-client selections must not enable them.
unset FOREVER_CASC_CLIENT FOREVER_CASC_EXPECTED_BUILD FOREVER_CASC_PROBE FOREVER_DB2_INSPECT FOREVER_DB2_EXTRACT
cd -- "$RCE_OPENCODE_ROOT"

# Workstation's already audited native reference and local development headers.
export RCE_FOREVER_CASC_REFERENCE="${RCE_FOREVER_CASC_REFERENCE:-$RCE_OPENCODE_ROOT/../rustycore/target/forever-cpp-reference}"
rce_openssl_root="$RCE_OPENCODE_WORK/cache/devdeps/openssl-3.5.5/extracted/usr"
if [[ -f "$rce_openssl_root/include/openssl/ssl.h" ]]; then
  export RCE_OPENSSL_INCLUDE="${RCE_OPENSSL_INCLUDE:-$rce_openssl_root/include}"
  export RCE_OPENSSL_ARCH_INCLUDE="${RCE_OPENSSL_ARCH_INCLUDE:-$rce_openssl_root/include/x86_64-linux-gnu}"
  export RCE_OPENSSL_CRYPTO="${RCE_OPENSSL_CRYPTO:-/lib/x86_64-linux-gnu/libcrypto.so.3}"
fi
# This selects a path only; ordinary checks still unset all live-test selectors.
export RCE_OPENCODE_CLIENT="${RCE_OPENCODE_CLIENT:-/home/joe/Games/battlenet/drive_c/Program Files (x86)/World of Warcraft}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

# Real-client outputs must also be outside the enclosing wow Git repository.
export RCE_OPENCODE_RUN_ROOT="${RCE_OPENCODE_RUN_ROOT:-${HOME}/.local/share/rustycore-extractor-work/runs/candidate}"
