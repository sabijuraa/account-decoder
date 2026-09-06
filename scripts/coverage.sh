#!/usr/bin/env bash
#
# Line coverage for the crates that carry the correctness burden.
#
# Uses the toolchain's own LLVM instrumentation rather than a coverage crate, so
# it needs nothing but `rustup component add llvm-tools-preview`. Integration
# tests are included, which matters here: the decoders are exercised against
# real mainnet fixtures from a separate test crate, not only by unit tests.
#
# Usage: scripts/coverage.sh [report|summary]
set -euo pipefail

cd "$(dirname "$0")/.."

LLVM_BIN="$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin"
PROFDATA="$LLVM_BIN/llvm-profdata"
COV="$LLVM_BIN/llvm-cov"
OUT="target/coverage"

if [ ! -x "$PROFDATA" ]; then
    echo "llvm-tools-preview is not installed: rustup component add llvm-tools-preview" >&2
    exit 1
fi

rm -rf "$OUT"
mkdir -p "$OUT"

export CARGO_INCREMENTAL=0
export RUSTFLAGS="-C instrument-coverage"
export LLVM_PROFILE_FILE="$PWD/$OUT/account-decoder-%p-%m.profraw"

# Build first and record the binaries, then run them. Doing it the other way
# round lets the second cargo invocation relink, and llvm-cov then reports
# "profile data may be out of date" and silently attributes nothing to the
# rebuilt objects — which reads as a plausible low coverage number rather than
# as the error it is.
echo "--- building the suite with instrumentation ---"
BINARIES=()
while IFS= read -r file; do
    BINARIES+=(-object "$file")
done < <(
    cargo test --workspace --no-run --message-format=json 2>/dev/null \
        | grep -o '"executable":"[^"]*"' | cut -d'"' -f4 | grep -v null
)
echo "${#BINARIES[@]} test binaries" | sed 's/ / objects, /'

echo "--- running the suite ---"
cargo test --workspace --no-fail-fast 2>&1 | tail -3

"$PROFDATA" merge -sparse "$OUT"/*.profraw -o "$OUT/account-decoder.profdata"

IGNORE='(/\.cargo/registry/|/rustc/|/tests/|target/debug/build/)'

case "${1:-summary}" in
    report)
        "$COV" show "${BINARIES[@]}" \
            --instr-profile="$OUT/account-decoder.profdata" \
            --ignore-filename-regex="$IGNORE" \
            --format=html --output-dir="$OUT/html"
        echo "HTML report: $OUT/html/index.html"
        ;;
    *)
        "$COV" report "${BINARIES[@]}" \
            --instr-profile="$OUT/account-decoder.profdata" \
            --ignore-filename-regex="$IGNORE" \
            --show-region-summary=false
        ;;
esac
