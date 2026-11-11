#!/bin/sh
# The project's checks (AGENTS.md, "Automated checks").
#   tools/check.sh fast   format, lint, type-check: ends every turn that changed files
#   tools/check.sh full   fast, then the tests and the release build: ends every session that
#                         changed code
# Clippy warnings are fatal: the code has none, and each new one is fixed where it appears.
set -eu
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"

case "${1:-fast}" in
fast | full) ;;
*)
    echo "usage: tools/check.sh [fast|full]" >&2
    exit 2
    ;;
esac

cargo fmt
dprint fmt
cargo clippy --all-targets -- -D warnings

if [ "${1:-fast}" = full ]; then
    cargo test --release
    cargo build --release
fi
