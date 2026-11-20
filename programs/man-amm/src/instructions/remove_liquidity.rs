use anchor_lang::prelude::*;
use anchor_spl::token::{
    burn, transfer, Burn, Mint, Token, TokenAccount, Transfer, TransferChecked,
};

use crate::{error::AmmError, withdraw_amounts, Pool, AUTHORITY_SEED, POOL_SEED};

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
		seeds = [POOL_SEED, pool.mint_a.key().as_ref(), pool.mint_b.key().as_ref()],
		bump = pool.pool_bump,
		has_one = vault_a,
		has_one = vault_b,
		has_one = lp_mint,
	)]
    pub pool: Account<'info, Pool>,

    /// CHECK: PDA derived from the pool key via seeds below. It holds no data
    /// and only acts as the signing authority for the vaults and LP mint.
    #[account(
        seeds = [AUTHORITY_SEED, pool.key().as_ref()],
        bump = pool.authority_bump
    )]
    pub pool_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub vault_a: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault_b: Account<'info, TokenAccount>,

    #[account(mut)]
    pub lp_mint: Account<'info, Mint>,

    #[account(
		mut,
		token::mint = pool.mint_a,
		token::authority = user,
	)]
    pub user_token_a: Account<'info, TokenAccount>,

    #[account(
		mut,
		token::mint = pool.mint_b,
		token::authority = user,
	)]
    pub user_token_b: Account<'info, TokenAccount>,

    #[account(
		mut,
		token::mint = lp_mint,
		token::authority = user,
	)]
    pub user_lp: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn remove_liquidity(
    ctx: Context<RemoveLiquidity>,
    lp_amount: u64,
    min_a: u64,
    min_b: u64,
) -> Result<()> {
    require!(lp_amount > 0, AmmError::ZeroAmount);

    let reserve_a = ctx.accounts.vault_a.amount;
    let reserve_b = ctx.accounts.vault_b.amount;
    let lp_supply = ctx.accounts.lp_mint.supply;

    let (out_a, out_b) = withdraw_amounts(lp_amount, lp_supply, reserve_a, reserve_b)?;

    require!(out_a >= min_a && out_b >= min_b, AmmError::SlippageExceeded);
    require!(out_a > 0 && out_b > 0, AmmError::ZeroOutput);

    burn(
        CpiContext::new(
            ctx.accounts.pool_authority.key(),
            Burn {
                mint: ctx.accounts.lp_mint.to_account_info(),
                from: ctx.accounts.user_lp.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        lp_amount,
    )?;

    let pool_key = ctx.accounts.pool.key();
    let signer_seeds: &[&[&[u8]]] = &[&[
        AUTHORITY_SEED,
        pool_key.as_ref(),
        &[ctx.accounts.pool.authority_bump],
    ]];

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.vault_a.to_account_info(),
                to: ctx.accounts.user_token_a.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
            signer_seeds,
        ),
        out_a,
    )?;

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.vault_b.to_account_info(),
                to: ctx.accounts.user_token_b.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
            signer_seeds,
        ),
        out_b,
    )?;

    Ok(())
}
