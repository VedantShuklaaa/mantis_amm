use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("mint_a must be strictly less than mint_b (sorted, and not equal)")]
    MintsNotSorted,
    #[msg("fee_bps exceeds the maximum allowed")]
    FeeTooHigh,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Initial deposit too small (must exceed minimum liquidity)")]
    InsufficientLiquidity,
    #[msg("Pool has no liquidity")]
    EmptyPool,
    #[msg("Output amount would be zero")]
    ZeroOutput,
}
