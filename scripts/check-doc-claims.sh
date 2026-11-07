#!/usr/bin/env bash
set -euo pipefail

protocol_version="$(sed -n 's/^pub const PROTOCOL_VERSION: u16 = \([0-9][0-9]*\);/\1/p' crates/ujani-core/src/peer_protocol.rs)"
if [[ "$protocol_version" != "2" ]]; then
  echo "Expected peer protocol v2; found '$protocol_version'." >&2
  exit 1
fi

if grep -RniE 'peer[- ]envelope v2.{0,100}(no peer enrollment|without peer enrollment)|peer protocol v2 primitives do not provide.{0,80}persistent replay' docs README.md; then
  echo "Documentation contains stale peer-protocol capability claims." >&2
  exit 1
fi

if grep -Rni 'Fortress.*OS-enforced isolation is implemented' docs README.md; then
  echo "Do not claim full application-level OS isolation from worker-only controls." >&2
  exit 1
fi

echo "Documentation claims are consistent with the current peer protocol and Fortress boundary."
