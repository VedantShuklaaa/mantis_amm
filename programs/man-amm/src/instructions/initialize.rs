use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{
    error::AmmError, Pool, AUTHORITY_SEED, LP_MINT_SEED, MAX_FEE_BPS, POOL_SEED, VAULT_SEED,
};

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        space = Pool::LEN,
        seeds = [POOL_SEED, mint_a.key().as_ref(), mint_b.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    /// CHECK: PDA derived from the pool key via seeds below. It holds no data
    /// and only acts as the signing authority for the vaults and LP mint.
    #[account(
        seeds = [AUTHORITY_SEED, pool.key().as_ref()],
        bump
    )]
    pub pool_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        seeds = [VAULT_SEED, pool.key().as_ref(), mint_a.key().as_ref()],
        bump,
        token::mint = mint_a,
        token::authority = pool_authority,
    )]
    pub vault_a: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = payer,
        seeds = [VAULT_SEED, pool.key().as_ref(), mint_b.key().as_ref()],
        bump,
        token::mint = mint_b,
        token::authority = pool_authority,
    )]
    pub vault_b: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = payer,
        seeds = [LP_MINT_SEED, pool.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = pool_authority,
    )]
    pub lp_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn initialize_pool(ctx: Context<InitializePool>, fee_bps: u16) -> Result<()> {
    require!(
        ctx.accounts.mint_a.key() < ctx.accounts.mint_b.key(),
        AmmError::MintsNotSorted
    );
    require!(fee_bps <= MAX_FEE_BPS, AmmError::FeeTooHigh);

    let pool = &mut ctx.accounts.pool;
    pool.mint_a = ctx.accounts.mint_a.key();
    pool.mint_b = ctx.accounts.mint_b.key();
    pool.vault_a = ctx.accounts.vault_a.key();
    pool.vault_b = ctx.accounts.vault_b.key();
    pool.lp_mint = ctx.accounts.lp_mint.key();
    pool.fee_bps = fee_bps;
    pool.pool_bump = ctx.bumps.pool;
    pool.authority_bump = ctx.bumps.pool_authority;

    Ok(())
}
