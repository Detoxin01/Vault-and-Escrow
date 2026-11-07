#!/usr/bin/env bash
set -euo pipefail

export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"
export PATH="$HOME/.avm/bin:$HOME/.local/share/solana/install/active_release/bin:$HOME/.cargo/bin:$PATH"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECTS=("anchor-vault" "anchor-escrow" "anchor-nc-vault")

# Use a fast WSL-native cache for node_modules if running off /mnt/c
NM_CACHE="$HOME/.cache/vault-escrow-nm"
if [[ "$ROOT_DIR" == /mnt/* ]]; then
  mkdir -p "$NM_CACHE"
  cp -f "$ROOT_DIR/anchor-vault/package.json" "$NM_CACHE/package.json"
  if [ ! -d "$NM_CACHE/node_modules/litesvm" ]; then
    (cd "$NM_CACHE" && yarn install --prefer-offline)
  fi
fi

for proj in "${PROJECTS[@]}"; do
  echo ""
  echo "========================================================================"
  echo " Running TypeScript + LiteSVM tests: $proj"
  echo "========================================================================"
  cd "$ROOT_DIR/$proj"
  if [[ "$ROOT_DIR" == /mnt/* ]]; then
    NODE_OPTIONS="--no-warnings" NODE_PATH="$NM_CACHE/node_modules" \
      "$NM_CACHE/node_modules/.bin/ts-mocha" \
      -p ./tsconfig.json -t 1000000 "tests/**/*.ts"
  else
    [ -d node_modules ] || yarn install --prefer-offline
    NODE_OPTIONS="--no-warnings" yarn test
  fi
done

echo ""
echo "All Vault, Timed Conditional Escrow, and Non-Custodial Vault test suites passed!"
