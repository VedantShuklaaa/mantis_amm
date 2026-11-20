use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Pool {
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub lp_mint: Pubkey,
    pub fee_bps: u16,
    pub pool_bump: u8,
    pub authority_bump: u8,
}

impl Pool {
    // 8 discriminator + InitSpace (32*5 + 2 + 1 + 1 = 164) = 172 bytes
    pub const LEN: usize = 8 + Self::INIT_SPACE;
}
