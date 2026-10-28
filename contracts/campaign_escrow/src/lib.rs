#![no_std]
#![allow(clippy::too_many_arguments)]

//! Campaign Escrow Contract with Multi-Signature Governance and Atomic Commission Disbursements.
//! Built natively on Stellar Soroban with persistent storage auto-bump TTL strategies.

#[cfg(test)]
mod test;

use soroban_sdk::{
    Address, Env, Vec, contract, contracterror, contractevent, contractimpl, contracttype, token,
};

/// Basis points divisor (100% = 10,000 bps).
pub const BPS_DIVISOR: i128 = 10_000;
/// Maximum allowed commission (15% = 1,500 bps).
pub const MAX_COMMISSION_BPS: u32 = 1_500;
/// Persistent entries are refreshed to ~31 days of ledger lifespan (~535,680 ledgers at 5s/ledger).
pub const PERSISTENT_TTL_EXTEND_TO: u32 = 535_680;
/// Persistent TTL threshold to trigger extension (~5.7 days of ledgers).
pub const PERSISTENT_TTL_THRESHOLD: u32 = 100_000;

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum EscrowError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    InvalidThreshold = 4,
    InvalidCommission = 5,
    InvalidDeadline = 6,
    CampaignAlreadyExists = 7,
    CampaignNotFound = 8,
    CampaignNotPending = 9,
    CampaignNotActive = 10,
    CampaignExpired = 11,
    CampaignNotExpired = 12,
    TargetAlreadyReached = 13,
    TargetNotReached = 14,
    AlreadyApproved = 15,
    NotAuthorizedSigner = 16,
    ZeroAmount = 17,
    InsufficientBalance = 18,
    ReentrantCall = 19,
    ArithmeticOverflow = 20,
    NoDepositFound = 21,
    CampaignAlreadyFinalized = 22,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum CampaignStatus {
    PendingApproval = 0,
    Active = 1,
    Finalized = 2,
    Failed = 3,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowInitialized {
    pub admin: Address,
    pub treasury: Address,
    pub threshold: u32,
    pub signers_count: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignCreated {
    pub campaign_id: u32,
    pub creator: Address,
    pub beneficiary: Address,
    pub token: Address,
    pub target_amount: i128,
    pub deadline: u64,
    pub commission_bps: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignApproved {
    pub campaign_id: u32,
    pub signer: Address,
    pub approval_count: u32,
    pub threshold: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignActivated {
    pub campaign_id: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsDeposited {
    pub campaign_id: u32,
    pub investor: Address,
    pub amount: i128,
    pub total_raised: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommissionDisbursed {
    pub campaign_id: u32,
    pub treasury: Address,
    pub commission_amount: i128,
    pub net_payout: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignFinalized {
    pub campaign_id: u32,
    pub beneficiary: Address,
    pub net_amount: i128,
    pub commission_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundClaimed {
    pub campaign_id: u32,
    pub investor: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    State,
    Signer(Address),
    Campaign(u32),
    Approval(u32, Address),
    Deposit(u32, Address),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowState {
    pub admin: Address,
    pub treasury: Address,
    pub threshold: u32,
    pub total_signers: u32,
    pub entered: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Campaign {
    pub id: u32,
    pub creator: Address,
    pub beneficiary: Address,
    pub token: Address,
    pub target_amount: i128,
    pub total_raised: i128,
    pub deadline: u64,
    pub commission_bps: u32,
    pub approval_count: u32,
    pub status: CampaignStatus,
}

#[contract]
pub struct CampaignEscrow;

#[contractimpl]
impl CampaignEscrow {
    /// Initializes the Escrow with protocol governance, platform treasury, and initial multi-sig signers.
    pub fn initialize(
        env: Env,
        admin: Address,
        treasury: Address,
        threshold: u32,
        signers: Vec<Address>,
    ) {
        let state_key = DataKey::State;
        if env.storage().persistent().has(&state_key) {
            soroban_sdk::panic_with_error!(&env, EscrowError::AlreadyInitialized);
        }
        admin.require_auth();

        let signers_len = signers.len();
        if threshold == 0 || threshold > signers_len {
            soroban_sdk::panic_with_error!(&env, EscrowError::InvalidThreshold);
        }

        // Register authorized multi-signature signers
        for i in 0..signers_len {
            let signer = signers.get(i).unwrap();
            let signer_key = DataKey::Signer(signer.clone());
            env.storage().persistent().set(&signer_key, &true);
            env.storage().persistent().extend_ttl(
                &signer_key,
                PERSISTENT_TTL_THRESHOLD,
                PERSISTENT_TTL_EXTEND_TO,
            );
        }

        let state = EscrowState {
            admin: admin.clone(),
            treasury: treasury.clone(),
            threshold,
            total_signers: signers_len,
            entered: false,
        };
        store_state(&env, &state);

        EscrowInitialized {
            admin,
            treasury,
            threshold,
            signers_count: signers_len,
        }
        .publish(&env);
    }

    /// Proposes a new fundraising or yield distribution campaign requiring multi-sig activation.
    pub fn create_campaign(
        env: Env,
        campaign_id: u32,
        creator: Address,
        beneficiary: Address,
        token: Address,
        target_amount: i128,
        duration_seconds: u64,
        commission_bps: u32,
    ) {
        creator.require_auth();
        let state = load_state(&env);
        ensure_not_entered(&env, &state);

        if target_amount <= 0 {
            soroban_sdk::panic_with_error!(&env, EscrowError::ZeroAmount);
        }
        if commission_bps > MAX_COMMISSION_BPS {
            soroban_sdk::panic_with_error!(&env, EscrowError::InvalidCommission);
        }
        if duration_seconds == 0 {
            soroban_sdk::panic_with_error!(&env, EscrowError::InvalidDeadline);
        }

        let campaign_key = DataKey::Campaign(campaign_id);
        if env.storage().persistent().has(&campaign_key) {
            soroban_sdk::panic_with_error!(&env, EscrowError::CampaignAlreadyExists);
        }

        // Verify token validity
        token::Client::new(&env, &token).decimals();

        let now = env.ledger().timestamp();
        let deadline = now.checked_add(duration_seconds).unwrap_or_else(|| {
            soroban_sdk::panic_with_error!(&env, EscrowError::ArithmeticOverflow)
        });

        let campaign = Campaign {
            id: campaign_id,
            creator: creator.clone(),
            beneficiary: beneficiary.clone(),
            token: token.clone(),
            target_amount,
            total_raised: 0,
            deadline,
            commission_bps,
            approval_count: 0,
            status: CampaignStatus::PendingApproval,
        };
        store_campaign(&env, &campaign);

        CampaignCreated {
            campaign_id,
            creator,
            beneficiary,
            token,
            target_amount,
            deadline,
            commission_bps,
        }
        .publish(&env);
    }

    /// Signs an approval for a pending campaign. When quorum is reached, campaign automatically activates.
    pub fn approve_campaign(env: Env, campaign_id: u32, signer: Address) {
        signer.require_auth();
        let state = load_state(&env);
        ensure_not_entered(&env, &state);

        // Verify signer is in authorized signers set
        let signer_key = DataKey::Signer(signer.clone());
        let is_authorized: bool = env.storage().persistent().get(&signer_key).unwrap_or(false);
        if !is_authorized {
            soroban_sdk::panic_with_error!(&env, EscrowError::NotAuthorizedSigner);
        }

        let mut campaign = load_campaign(&env, campaign_id);
        if campaign.status != CampaignStatus::PendingApproval {
            soroban_sdk::panic_with_error!(&env, EscrowError::CampaignNotPending);
        }

        // Prevent double signing
        let approval_key = DataKey::Approval(campaign_id, signer.clone());
        if env.storage().persistent().has(&approval_key) {
            soroban_sdk::panic_with_error!(&env, EscrowError::AlreadyApproved);
        }

        env.storage().persistent().set(&approval_key, &true);
        env.storage().persistent().extend_ttl(
            &approval_key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        campaign.approval_count = campaign.approval_count.checked_add(1).unwrap_or_else(|| {
            soroban_sdk::panic_with_error!(&env, EscrowError::ArithmeticOverflow)
        });

        CampaignApproved {
            campaign_id,
            signer,
            approval_count: campaign.approval_count,
            threshold: state.threshold,
        }
        .publish(&env);

        if campaign.approval_count >= state.threshold {
            campaign.status = CampaignStatus::Active;
            CampaignActivated { campaign_id }.publish(&env);
        }

        store_campaign(&env, &campaign);
    }

    /// Deposits funds into an active campaign escrow.
    pub fn deposit(env: Env, campaign_id: u32, investor: Address, amount: i128) {
        investor.require_auth();
        let mut state = load_state(&env);
        ensure_not_entered(&env, &state);

        if amount <= 0 {
            soroban_sdk::panic_with_error!(&env, EscrowError::ZeroAmount);
        }

        let mut campaign = load_campaign(&env, campaign_id);
        if campaign.status != CampaignStatus::Active {
            soroban_sdk::panic_with_error!(&env, EscrowError::CampaignNotActive);
        }

        let now = env.ledger().timestamp();
        if now > campaign.deadline {
            soroban_sdk::panic_with_error!(&env, EscrowError::CampaignExpired);
        }

        // Record deposit
        let deposit_key = DataKey::Deposit(campaign_id, investor.clone());
        let current_deposit: i128 = env.storage().persistent().get(&deposit_key).unwrap_or(0);
        let updated_deposit = current_deposit.checked_add(amount).unwrap_or_else(|| {
            soroban_sdk::panic_with_error!(&env, EscrowError::ArithmeticOverflow)
        });

        env.storage()
            .persistent()
            .set(&deposit_key, &updated_deposit);
        env.storage().persistent().extend_ttl(
            &deposit_key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        campaign.total_raised = campaign
            .total_raised
            .checked_add(amount)
            .unwrap_or_else(|| {
                soroban_sdk::panic_with_error!(&env, EscrowError::ArithmeticOverflow)
            });

        store_campaign(&env, &campaign);

        // Reentrancy lock around transfer
        state.entered = true;
        store_state(&env, &state);

        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &campaign.token);
        if token_client.balance(&investor) < amount {
            soroban_sdk::panic_with_error!(&env, EscrowError::InsufficientBalance);
        }
        token_client.transfer(&investor, &contract_address, &amount);

        state.entered = false;
        store_state(&env, &state);

        FundsDeposited {
            campaign_id,
            investor,
            amount,
            total_raised: campaign.total_raised,
        }
        .publish(&env);
    }

    /// Finalizes a successful campaign, atomically routing the platform commission to treasury
    /// and the net balance to the beneficiary within Stellar's 5-second ledger finality.
    pub fn finalize_campaign(env: Env, campaign_id: u32) {
        let mut state = load_state(&env);
        ensure_not_entered(&env, &state);

        let mut campaign = load_campaign(&env, campaign_id);
        if campaign.status != CampaignStatus::Active {
            soroban_sdk::panic_with_error!(&env, EscrowError::CampaignNotActive);
        }

        let now = env.ledger().timestamp();
        let target_reached = campaign.total_raised >= campaign.target_amount;
        let expired = now > campaign.deadline;

        // Campaign can finalize if target is met or deadline elapsed with target met
        if !target_reached {
            if expired {
                campaign.status = CampaignStatus::Failed;
                store_campaign(&env, &campaign);
                soroban_sdk::panic_with_error!(&env, EscrowError::TargetNotReached);
            } else {
                soroban_sdk::panic_with_error!(&env, EscrowError::TargetNotReached);
            }
        }

        campaign.status = CampaignStatus::Finalized;
        store_campaign(&env, &campaign);

        // Calculate atomic commission
        let total_raised = campaign.total_raised;
        let commission_bps = campaign.commission_bps as i128;
        let commission_amount = total_raised.checked_mul(commission_bps).unwrap_or_else(|| {
            soroban_sdk::panic_with_error!(&env, EscrowError::ArithmeticOverflow)
        }) / BPS_DIVISOR;

        let net_payout = total_raised
            .checked_sub(commission_amount)
            .unwrap_or_else(|| {
                soroban_sdk::panic_with_error!(&env, EscrowError::ArithmeticOverflow)
            });

        // Reentrancy lock around disbursements
        state.entered = true;
        store_state(&env, &state);

        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &campaign.token);

        // 1. Disburse platform commission atomically to treasury
        if commission_amount > 0 {
            token_client.transfer(&contract_address, &state.treasury, &commission_amount);
            CommissionDisbursed {
                campaign_id,
                treasury: state.treasury.clone(),
                commission_amount,
                net_payout,
            }
            .publish(&env);
        }

        // 2. Disburse net proceeds atomically to beneficiary
        if net_payout > 0 {
            token_client.transfer(&contract_address, &campaign.beneficiary, &net_payout);
        }

        state.entered = false;
        store_state(&env, &state);

        CampaignFinalized {
            campaign_id,
            beneficiary: campaign.beneficiary,
            net_amount: net_payout,
            commission_amount,
        }
        .publish(&env);
    }

    /// Allows investors to claim a 100% full refund if campaign failed to reach target by deadline.
    pub fn claim_refund(env: Env, campaign_id: u32, investor: Address) -> i128 {
        investor.require_auth();
        let mut state = load_state(&env);
        ensure_not_entered(&env, &state);

        let mut campaign = load_campaign(&env, campaign_id);
        let now = env.ledger().timestamp();

        // Mark failed if past deadline without target
        if campaign.status == CampaignStatus::Active
            && now > campaign.deadline
            && campaign.total_raised < campaign.target_amount
        {
            campaign.status = CampaignStatus::Failed;
            store_campaign(&env, &campaign);
        }

        if campaign.status != CampaignStatus::Failed {
            soroban_sdk::panic_with_error!(&env, EscrowError::CampaignNotExpired);
        }

        let deposit_key = DataKey::Deposit(campaign_id, investor.clone());
        let deposited_amount: i128 = env.storage().persistent().get(&deposit_key).unwrap_or(0);
        if deposited_amount <= 0 {
            soroban_sdk::panic_with_error!(&env, EscrowError::NoDepositFound);
        }

        // Reset deposit
        env.storage().persistent().set(&deposit_key, &0i128);
        env.storage().persistent().extend_ttl(
            &deposit_key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );

        state.entered = true;
        store_state(&env, &state);

        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &campaign.token);
        token_client.transfer(&contract_address, &investor, &deposited_amount);

        state.entered = false;
        store_state(&env, &state);

        RefundClaimed {
            campaign_id,
            investor,
            amount: deposited_amount,
        }
        .publish(&env);

        deposited_amount
    }

    /// Returns the campaign details.
    pub fn get_campaign(env: Env, campaign_id: u32) -> Campaign {
        load_campaign(&env, campaign_id)
    }

    /// Returns the global escrow configuration (admin, treasury, threshold, total_signers).
    pub fn configuration(env: Env) -> (Address, Address, u32, u32) {
        let state = load_state(&env);
        (
            state.admin,
            state.treasury,
            state.threshold,
            state.total_signers,
        )
    }

    /// Returns an investor's deposit balance in a specific campaign.
    pub fn get_deposit(env: Env, campaign_id: u32, investor: Address) -> i128 {
        let key = DataKey::Deposit(campaign_id, investor);
        env.storage().persistent().get(&key).unwrap_or(0)
    }
}

pub fn load_state(env: &Env) -> EscrowState {
    let key = DataKey::State;
    let state = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, EscrowError::NotInitialized));
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
    env.storage()
        .instance()
        .extend_ttl(PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
    state
}

pub fn store_state(env: &Env, state: &EscrowState) {
    let key = DataKey::State;
    env.storage().persistent().set(&key, state);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
    env.storage()
        .instance()
        .extend_ttl(PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

pub fn load_campaign(env: &Env, campaign_id: u32) -> Campaign {
    let key = DataKey::Campaign(campaign_id);
    let campaign: Campaign = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, EscrowError::CampaignNotFound));
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
    campaign
}

pub fn store_campaign(env: &Env, campaign: &Campaign) {
    let key = DataKey::Campaign(campaign.id);
    env.storage().persistent().set(&key, campaign);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
    env.storage()
        .instance()
        .extend_ttl(PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

fn ensure_not_entered(env: &Env, state: &EscrowState) {
    if state.entered {
        soroban_sdk::panic_with_error!(env, EscrowError::ReentrantCall);
    }
}
