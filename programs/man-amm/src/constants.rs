use anchor_lang::prelude::*;

#[constant]
pub const POOL_SEED: &[u8] = b"pool";
#[constant]
pub const AUTHORITY_SEED: &[u8] = b"authority";

#[constant]
pub const VAULT_SEED: &[u8] = b"vault";

#[constant]
pub const LP_MINT_SEED: &[u8] = b"lp_mint";

#[constant]
pub const LOCKED_LP_SEED: &[u8] = b"locked_lp";

pub const BPS_DENOMINATOR: u64 = 10_000;
pub const MAX_FEE_BPS: u16 = 100; // 1% cap
pub const MINIMUM_LIQUIDITY: u64 = 1_000;
