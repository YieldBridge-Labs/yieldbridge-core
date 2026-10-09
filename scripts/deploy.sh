#!/usr/bin/env bash
set -e

# ==============================================================================
# YieldBridge Core: Testnet Deployment & Initialization Script
# ==============================================================================

NETWORK="${STELLAR_NETWORK:-testnet}"
SOURCE_ACCOUNT="${STELLAR_SOURCE_ACCOUNT:-admin}"
RPC_URL="${STELLAR_RPC_URL:-https://soroban-testnet.stellar.org}"

echo "==> [1/4] Building optimized Soroban contracts for target wasm32v1-none..."
cargo fmt --all -- --check
cargo test --workspace --all-targets

stellar contract build --package vault_core --target wasm32v1-none
stellar contract build --package stream_factory --target wasm32v1-none

VAULT_WASM="target/wasm32v1-none/release/vault_core.wasm"
FACTORY_WASM="target/wasm32v1-none/release/stream_factory.wasm"

echo "==> [2/4] Uploading contract WASMs to Stellar Testnet..."
VAULT_WASM_HASH=$(stellar contract install --wasm "$VAULT_WASM" --network "$NETWORK" --source "$SOURCE_ACCOUNT" --porcelain)
echo "    Vault Core WASM Hash: $VAULT_WASM_HASH"

FACTORY_WASM_HASH=$(stellar contract install --wasm "$FACTORY_WASM" --network "$NETWORK" --source "$SOURCE_ACCOUNT" --porcelain)
echo "    Stream Factory WASM Hash: $FACTORY_WASM_HASH"

echo "==> [3/4] Deploying Stream Factory Contract..."
FACTORY_CONTRACT_ID=$(stellar contract deploy \
  --wasm-hash "$FACTORY_WASM_HASH" \
  --network "$NETWORK" \
  --source "$SOURCE_ACCOUNT" \
  --porcelain)

echo "    Stream Factory Deployed ID: $FACTORY_CONTRACT_ID"

echo "==> [4/4] Initializing Factory / Test Vault..."
# Optional: Deploy a test vault via factory salt deployment call
SALT=$(openssl rand -hex 32 2>/dev/null || date +%s%N | sha256sum | head -c 64)
ADMIN_ADDRESS=$(stellar keys address "$SOURCE_ACCOUNT" 2>/dev/null || echo "")

echo ""
echo "========================================================================="
echo " DEPLOYMENT SUCCESSFUL — COPY THESE ENVIRONMENT VARIABLES TO YOUR APP:"
echo "========================================================================="
echo "NEXT_PUBLIC_STREAM_FACTORY_ID=$FACTORY_CONTRACT_ID"
echo "NEXT_PUBLIC_VAULT_TEMPLATE_HASH=$VAULT_WASM_HASH"
echo "NEXT_PUBLIC_STELLAR_NETWORK=$NETWORK"
echo "NEXT_PUBLIC_RPC_URL=$RPC_URL"
echo "========================================================================="
