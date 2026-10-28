#![no_std]

//! Public Rust interfaces for the YieldBridge Soroban contracts.

pub use campaign_escrow::{
    Campaign, CampaignEscrow, CampaignEscrowClient, CampaignStatus, EscrowError, EscrowState,
};
pub use stream_factory::{FactoryError, StreamFactory, StreamFactoryClient};
pub use vault_core::{DataKey, Investor, VaultError, VaultState, YieldVault, YieldVaultClient};
