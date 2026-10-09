# ==============================================================================
# YieldBridge Core: Testnet Deployment & Initialization Script (PowerShell)
# ==============================================================================
$ErrorActionPreference = "Stop"

$Network = if ($env:STELLAR_NETWORK) { $env:STELLAR_NETWORK } else { "testnet" }
$SourceAccount = if ($env:STELLAR_SOURCE_ACCOUNT) { $env:STELLAR_SOURCE_ACCOUNT } else { "admin" }
$RpcUrl = if ($env:STELLAR_RPC_URL) { $env:STELLAR_RPC_URL } else { "https://soroban-testnet.stellar.org" }

Write-Host "==> [1/4] Building optimized Soroban contracts for target wasm32v1-none..."
cargo fmt --all -- --check
cargo test --workspace --all-targets

$env:SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2 = "1"
cargo build --release --target wasm32v1-none --package vault_core
cargo build --release --target wasm32v1-none --package stream_factory

$VaultWasm = "target/wasm32v1-none/release/vault_core.wasm"
$FactoryWasm = "target/wasm32v1-none/release/stream_factory.wasm"

Write-Host "==> [2/4] Uploading contract WASMs to Stellar Testnet..."
$VaultWasmHash = (stellar contract install --wasm $VaultWasm --network $Network --source $SourceAccount --porcelain)
Write-Host "    Vault Core WASM Hash: $VaultWasmHash"

$FactoryWasmHash = (stellar contract install --wasm $FactoryWasm --network $Network --source $SourceAccount --porcelain)
Write-Host "    Stream Factory WASM Hash: $FactoryWasmHash"

Write-Host "==> [3/4] Deploying Stream Factory Contract..."
$FactoryContractId = (stellar contract deploy --wasm-hash $FactoryWasmHash --network $Network --source $SourceAccount --porcelain)
Write-Host "    Stream Factory Deployed ID: $FactoryContractId"

Write-Host "==> [4/4] Generating deployment summary..."
Write-Host ""
Write-Host "========================================================================="
Write-Host " DEPLOYMENT SUCCESSFUL — COPY THESE ENVIRONMENT VARIABLES TO YOUR APP:"
Write-Host "========================================================================="
Write-Host "NEXT_PUBLIC_STREAM_FACTORY_ID=$FactoryContractId"
Write-Host "NEXT_PUBLIC_VAULT_TEMPLATE_HASH=$VaultWasmHash"
Write-Host "NEXT_PUBLIC_STELLAR_NETWORK=$Network"
Write-Host "NEXT_PUBLIC_RPC_URL=$RpcUrl"
Write-Host "========================================================================="
