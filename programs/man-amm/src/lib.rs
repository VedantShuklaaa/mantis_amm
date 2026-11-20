pub mod constants;
pub mod error;
pub mod instructions;
pub mod math;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use math::*;
pub use state::*;

declare_id!("DwMeAF9UFscahiqeYKr1CmKhEwdp9TBxihGbcK5YqJFm");

#[program]
pub mod man_amm {
    use super::*;

    pub fn initialize_pool(ctx: Context<InitializePool>, fee_bps: u16) -> Result<()> {
        initialize::initialize_pool(ctx, fee_bps)
    }

    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        max_a: u64,
        max_b: u64,
        min_lp: u64,
    ) -> Result<()> {
        instructions::add_liquidity::add_liquidity(ctx, max_a, max_b, min_lp)
    }

    pub fn remove_liquidity(
        ctx: Context<RemoveLiquidity>,
        lp_amount: u64,
        min_a: u64,
        min_b: u64,
    ) -> Result<()> {
        instructions::remove_liquidity::remove_liquidity(ctx, lp_amount, min_a, min_b)
    }

    pub fn swap(ctx: Context<Swap>, amount_in: u64, min_out: u64, a_to_b: bool) -> Result<()> {
        instructions::swap::swap(ctx, amount_in, min_out, a_to_b)
    }
}
