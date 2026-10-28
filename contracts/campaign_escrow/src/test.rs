#![cfg(test)]

use super::*;
use soroban_sdk::{
    Address, Env, Vec,
    testutils::{Address as _, Ledger as _, storage::Persistent as _},
    token,
};

fn setup_escrow() -> (
    Env,
    Address,
    Address,
    Address,
    Address,
    Address,
    Address,
    CampaignEscrowClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let escrow_address = env.register(CampaignEscrow, ());
    let client = CampaignEscrowClient::new(&env, &escrow_address);

    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());
    signers.push_back(signer3.clone());

    // 2-of-3 threshold
    client.initialize(&admin, &treasury, &2, &signers);

    (
        env,
        admin,
        treasury,
        signer1,
        signer2,
        signer3,
        token_address,
        client,
    )
}

#[test]
fn test_escrow_initialization() {
    let (_env, admin, treasury, _s1, _s2, _s3, _token, client) = setup_escrow();
    let (cfg_admin, cfg_treasury, threshold, signers_count) = client.configuration();
    assert_eq!(cfg_admin, admin);
    assert_eq!(cfg_treasury, treasury);
    assert_eq!(threshold, 2);
    assert_eq!(signers_count, 3);
}

#[test]
fn test_campaign_lifecycle_with_multisig_and_atomic_commission() {
    let (env, _admin, treasury, s1, s2, _s3, token_address, client) = setup_escrow();

    let creator = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let investor1 = Address::generate(&env);
    let investor2 = Address::generate(&env);

    let token_admin = token::StellarAssetClient::new(&env, &token_address);
    let token_client = token::Client::new(&env, &token_address);

    // 500 bps = 5% commission
    let campaign_id = 1u32;
    let target = 10_000i128;
    let duration = 3600u64;
    let commission_bps = 500u32;

    client.create_campaign(
        &campaign_id,
        &creator,
        &beneficiary,
        &token_address,
        &target,
        &duration,
        &commission_bps,
    );

    let c = client.get_campaign(&campaign_id);
    assert_eq!(c.status, CampaignStatus::PendingApproval);
    assert_eq!(c.approval_count, 0);

    // First signer approves
    client.approve_campaign(&campaign_id, &s1);
    let c = client.get_campaign(&campaign_id);
    assert_eq!(c.status, CampaignStatus::PendingApproval);
    assert_eq!(c.approval_count, 1);

    // Second signer approves -> Quorum reached (2 of 3) -> Status becomes Active
    client.approve_campaign(&campaign_id, &s2);
    let c = client.get_campaign(&campaign_id);
    assert_eq!(c.status, CampaignStatus::Active);
    assert_eq!(c.approval_count, 2);

    // Fund investors
    token_admin.mint(&investor1, &6_000);
    token_admin.mint(&investor2, &4_000);

    // Investors deposit
    client.deposit(&campaign_id, &investor1, &6_000);
    client.deposit(&campaign_id, &investor2, &4_000);

    assert_eq!(client.get_deposit(&campaign_id, &investor1), 6_000);
    assert_eq!(client.get_deposit(&campaign_id, &investor2), 4_000);
    let c = client.get_campaign(&campaign_id);
    assert_eq!(c.total_raised, 10_000);

    // Finalize campaign:
    // Total = 10,000
    // Commission (5%) = 500 -> to treasury
    // Net = 9,500 -> to beneficiary
    client.finalize_campaign(&campaign_id);

    assert_eq!(token_client.balance(&treasury), 500);
    assert_eq!(token_client.balance(&beneficiary), 9_500);

    let c = client.get_campaign(&campaign_id);
    assert_eq!(c.status, CampaignStatus::Finalized);
}

#[test]
fn test_campaign_refund_on_failure() {
    let (env, _admin, _treasury, s1, s2, _s3, token_address, client) = setup_escrow();

    let creator = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let investor = Address::generate(&env);

    let token_admin = token::StellarAssetClient::new(&env, &token_address);
    let token_client = token::Client::new(&env, &token_address);

    let campaign_id = 2u32;
    client.create_campaign(
        &campaign_id,
        &creator,
        &beneficiary,
        &token_address,
        &10_000,
        &100,
        &200,
    );

    client.approve_campaign(&campaign_id, &s1);
    client.approve_campaign(&campaign_id, &s2);

    token_admin.mint(&investor, &3_000);
    client.deposit(&campaign_id, &investor, &3_000);
    assert_eq!(token_client.balance(&investor), 0);

    // Fast-forward past deadline
    env.ledger().set_timestamp(200);

    // Refund claimable
    let refunded = client.claim_refund(&campaign_id, &investor);
    assert_eq!(refunded, 3_000);
    assert_eq!(token_client.balance(&investor), 3_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #15)")]
fn test_double_approval_rejected() {
    let (_env, _admin, _treasury, s1, _s2, _s3, token_address, client) = setup_escrow();
    let creator = Address::generate(&_env);
    let beneficiary = Address::generate(&_env);

    client.create_campaign(
        &3,
        &creator,
        &beneficiary,
        &token_address,
        &5_000,
        &100,
        &100,
    );
    client.approve_campaign(&3, &s1);
    // Should panic with AlreadyApproved (15)
    client.approve_campaign(&3, &s1);
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")]
fn test_unauthorized_signer_rejected() {
    let (env, _admin, _treasury, _s1, _s2, _s3, token_address, client) = setup_escrow();
    let creator = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let attacker = Address::generate(&env);

    client.create_campaign(
        &4,
        &creator,
        &beneficiary,
        &token_address,
        &5_000,
        &100,
        &100,
    );
    client.approve_campaign(&4, &attacker);
}

#[test]
fn test_escrow_persistent_ttl_auto_bump() {
    let (env, admin, _treasury, _s1, _s2, _s3, token_address, client) = setup_escrow();
    let creator = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    client.create_campaign(
        &99,
        &creator,
        &beneficiary,
        &token_address,
        &5_000,
        &100,
        &100,
    );

    let state_key = DataKey::State;
    let campaign_key = DataKey::Campaign(99);

    let seq = env.ledger().sequence();
    env.ledger().set_sequence_number(seq + 450_000);

    // Call get_campaign which extends TTL
    let _ = client.get_campaign(&99);

    let (admin_cfg, _treasury_cfg, _th, _signers) = client.configuration();
    assert_eq!(admin_cfg, admin);

    let (state_ttl, camp_ttl) = env.as_contract(&client.address, || {
        (
            env.storage().persistent().get_ttl(&state_key),
            env.storage().persistent().get_ttl(&campaign_key),
        )
    });
    assert!(state_ttl >= PERSISTENT_TTL_EXTEND_TO);
    assert!(camp_ttl >= PERSISTENT_TTL_EXTEND_TO);
}
