#!/usr/bin/env bash
# Publish the sotf-* crates to crates.io, dependencies first.
#
# Usage:
#   scripts/release.sh [--execute] [--yes] [LEVEL...] [GROUP...]
#
# Without --execute this is a dry run (cargo-release prints what it would do).
# With --execute it publishes for real. Optional LEVEL args (L0, L1, L2)
# restrict the run to those dependency levels; optional GROUP args (crate
# directory names) restrict it to those groups.
#
# Publish order (each level needs the previous one live on crates.io):
#   L0: leaves (no intra-collection dependencies)
#   L1: derive_more nih-plug tract
#   L2: gpui
#
# Prerequisites:
#   - cargo-release installed (`cargo install cargo-release`)
#   - crates.io token authorized for the sotf-* crates (CARGO_REGISTRY_TOKEN)
#   - clean git worktree (cargo-release refuses to run on a dirty tree)
#
# Notes:
#   - Crates marked `publish = false` (examples, harnesses, benches, xtask
#     helpers) are skipped automatically.
#   - sotf-derive_more is published with --no-verify: its upstream sources
#     intentionally fail a default-feature build ("at least one derive feature
#     must be enabled"), which is also how upstream ships it. The crate is
#     covered by `cargo check --features full` instead (see RELEASE.md).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MODE=""
CONFIRM=()
LEVELS=()
WANT_GROUPS=()
for arg in "$@"; do
    case "$arg" in
        --execute) MODE="--execute" ;;
        --yes) CONFIRM=(--no-confirm) ;;
        L0|L1|L2) LEVELS+=("$arg") ;;
        -*) echo "unknown argument: $arg" >&2; exit 2 ;;
        *) WANT_GROUPS+=("$arg") ;;
    esac
done
if [ "${#LEVELS[@]}" -eq 0 ]; then
    LEVELS=(L0 L1 L2)
fi

L0=(async-task baseview block coreaudio-rs cosmic-text derive_more-impl dst-decoder fontconfig-parser headers librespot metaheuristics-nature-rs nalgebra nnnoiseless objc oo7 predicates rdsd2pcm rfd rubato rust-assert-no-alloc rusty-fork scap stacker tflitec vst3-sys wgpu xim-rs zed-font-kit)
L1=(derive_more nih-plug tract)
L2=(gpui)
WORKSPACES=" gpui wgpu tract librespot nalgebra xim-rs vst3-sys nih-plug "

echo "==> preflight: scripts/verify.py"
python3 scripts/verify.py

for level in "${LEVELS[@]}"; do
    if [ -n "$MODE" ]; then
        echo "==> level $level ($MODE)"
    else
        echo "==> level $level (dry run)"
    fi
    eval "groups=(\"\${$level[@]}\")"
    for group in "${groups[@]}"; do
        if [ "${#WANT_GROUPS[@]}" -gt 0 ]; then
            skip=1
            for want in "${WANT_GROUPS[@]}"; do
                if [ "$want" = "$group" ]; then
                    skip=0
                fi
            done
            if [ "$skip" -eq 1 ]; then
                continue
            fi
        fi
        echo "---- $group"
        args=(publish)
        if [ -n "$MODE" ]; then
            args+=("$MODE")
        fi
        if [ "${#CONFIRM[@]}" -gt 0 ]; then
            args+=("${CONFIRM[@]}")
        fi
        if [[ "$WORKSPACES" == *" $group "* ]]; then
            args+=(--workspace)
        fi
        if [ "$group" = "derive_more" ]; then
            # Upstream by-design guard fails default-feature builds; the crate
            # is verified with --features full instead (see header comment).
            args+=(--no-verify)
        fi
        log="$(mktemp)"
        if (cd "$group" && cargo release "${args[@]}" 2>&1 | tee "$log"); then
            rm -f "$log"
        elif [ -z "$MODE" ] && grep -q "uncommitted changes detected" "$log" \
            && grep -q "aborting upload due to dry run" "$log"; then
            # Dry runs are for pre-commit smoke tests: the flow itself
            # completed, only the clean-tree gate fired. A real release
            # (--execute) keeps the strict gate and aborts here instead.
            echo "(dry run completed despite dirty tree; commit first for a clean run)"
            rm -f "$log"
        else
            echo "release failed for $group; log: $log" >&2
            exit 1
        fi
    done
done
if [ -n "$MODE" ]; then
    echo "done ($MODE)."
else
    echo "done (dry run)."
fi
