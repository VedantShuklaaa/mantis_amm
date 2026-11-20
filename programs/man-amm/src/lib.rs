pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("DwMeAF9UFscahiqeYKr1CmKhEwdp9TBxihGbcK5YqJFm");

#[program]
pub mod man_amm {
    use super::*;

    pub fn initialize_pool(ctx: Context<InitializePool>, fee_bps: u16) -> Result<()> {
        initialize::initialize_pool(ctx, fee_bps)
    }
}
