use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("mint_a must be strictly less than mint_b (sorted, and not equal)")]
    MintsNotSorted,
    #[msg("fee_bps exceeds the maximum allowed")]
    FeeTooHigh,
    #[msg("Math overflow")]
    MathOverflow,
}
