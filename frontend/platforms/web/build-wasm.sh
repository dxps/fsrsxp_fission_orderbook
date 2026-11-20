#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)
PROJECT_DIR=$(cd -- "$SCRIPT_DIR/../.." && pwd)
PROFILE="${FISSION_WEB_PROFILE:-dev}"
SIMD_MODE="${FISSION_WEB_WASM_SIMD:-simd}"
SIMD_MODE=$(printf '%s' "$SIMD_MODE" | tr '[:upper:]' '[:lower:]')

case "$SIMD_MODE" in
  1|true|yes|on|simd|simd128)
    GLOBAL_FLAGS="${CARGO_ENCODED_RUSTFLAGS:-${RUSTFLAGS:-}}"
    if [[ -n "$GLOBAL_FLAGS" ]]; then
      if [[ "$GLOBAL_FLAGS" != *"+simd128"* || "$GLOBAL_FLAGS" == *"-simd128"* ]]; then
        printf '%s\n' "Global Rust flags override target-specific Web flags; add +simd128 to them or set FISSION_WEB_WASM_SIMD=scalar." >&2
        exit 2
      fi
    else
      TARGET_FLAGS="${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-}"
      if [[ "$TARGET_FLAGS" == *"target-feature="* ]]; then
        if [[ "$TARGET_FLAGS" != *"+simd128"* ]]; then
          printf '%s\n' "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS already sets target-feature; include +simd128 or set FISSION_WEB_WASM_SIMD=scalar." >&2
          exit 2
        fi
      else
        export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="${TARGET_FLAGS:+$TARGET_FLAGS }-C target-feature=+simd128"
      fi
    fi
    ;;
  0|false|no|off|scalar)
    GLOBAL_FLAGS="${CARGO_ENCODED_RUSTFLAGS:-${RUSTFLAGS:-}}"
    if [[ "$GLOBAL_FLAGS" == *"+simd128"* && "$GLOBAL_FLAGS" != *"-simd128"* ]]; then
      printf '%s\n' "Global Rust flags enable simd128; remove that flag for a scalar build." >&2
      exit 2
    fi
    if [[ "${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-}" == *"+simd128"* ]]; then
      printf '%s\n' "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS enables simd128; remove that flag for a scalar build." >&2
      exit 2
    fi
    ;;
  *)
    printf '%s\n' "FISSION_WEB_WASM_SIMD must be 'simd' (default) or 'scalar'." >&2
    exit 2
    ;;
esac

BUILD_ARGS=(build "$PROJECT_DIR" --target web --out-dir "$SCRIPT_DIR/pkg")

if [[ "$PROFILE" == "release" ]]; then
  BUILD_ARGS+=(--release)
else
  BUILD_ARGS+=(--dev)
fi

wasm-pack "${BUILD_ARGS[@]}"
