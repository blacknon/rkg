#!/usr/bin/env bash

set -euo pipefail

cargo build --locked --quiet

BIN="target/debug/rkg"
if [[ ! -x "$BIN" && -f "${BIN}.exe" ]]; then
  BIN="${BIN}.exe"
fi

if [[ ! -f "$BIN" ]]; then
  echo "smoke: built binary not found: $BIN" >&2
  exit 1
fi

assert_output() {
  local name="$1"
  local input="$2"
  local expected="$3"
  shift 3

  local actual
  actual="$(printf '%s' "$input" | "$BIN" "$@")"

  if [[ "$actual" != "$expected" ]]; then
    echo "smoke failed: $name" >&2
    echo "--- expected ---" >&2
    printf '%s\n' "$expected" >&2
    echo "--- actual ---" >&2
    printf '%s\n' "$actual" >&2
    exit 1
  fi

  echo "smoke passed: $name"
}

# Representative README-driven checks for released platforms.
assert_output \
  "record quick start shorthand" \
  $'A,10;tokyo\nB:20;osaka\n' \
  $'A|10|tokyo\nB|20|osaka' \
  -F '[,:;]' \
  'r.p:1,2,3.ofs=|'

assert_output \
  "grid rotate shorthand" \
  $'abc\ndef\nghi\n' \
  $'cba\nfed\nihg' \
  'g.t.rt:r'

assert_output \
  "record to grid pipeline" \
  $'A 10\nB 20\n' \
  $'AB\n--\n12\n00' \
  'r.p:1,2.ofs=- | g.t'

assert_output \
  "grid pattern mark" \
  $'.......\n.......\n.XOOOX.\n.......\n.......\n' \
  $'.......\n.......\n.X***X.\n.......\n.......' \
  'g.m("X","O","X","*")'
