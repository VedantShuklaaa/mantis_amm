use crate::{
    amounts_for_deposit, error::AmmError, initial_lp, lp_for_deposit, Pool, AUTHORITY_SEED,
    POOL_SEED,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct AddLiquidity<'info> {
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

pub fn add_liquidity(
    ctx: Context<AddLiquidity>,
    max_a: u64,
    max_b: u64,
    min_lp: u64,
) -> Result<()> {
    require!(max_a > 0, AmmError::MathOverflow);
    require!(max_b > 0, AmmError::MathOverflow);

    let reserve_a = ctx.accounts.vault_a.amount;
    let reserve_b = ctx.accounts.vault_b.amount;
    let lp_supply = ctx.accounts.lp_mint.supply;

    let (amount_a, amount_b, lp_to_mint) = if lp_supply == 0 {
        let lp = initial_lp(max_a, max_b)?;
        (max_a, max_b, lp)
    } else {
        let (a, b) = amounts_for_deposit(max_a, max_b, reserve_a, reserve_b)?;
        let lp = lp_for_deposit(a, b, reserve_a, reserve_b, lp_supply)?;
        (a, b, lp)
    };

    require!(lp_to_mint > 0, AmmError::ZeroAmount);
    require!(lp_to_mint >= min_lp, AmmError::SlippageExceeded);

    // 1. user -> vaults (user signs, no PDA seeds)
    transfer(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.user_token_a.to_account_info(),
                to: ctx.accounts.vault_a.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_a,
    )?;

    transfer(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.user_token_b.to_account_info(),
                to: ctx.accounts.vault_b.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_b,
    )?;

    // 2. mint LP to user (pool authority signs)
    let pool_key = ctx.accounts.pool.key();
    let signer_seeds: &[&[&[u8]]] = &[&[
        AUTHORITY_SEED,
        pool_key.as_ref(),
        &[ctx.accounts.pool.authority_bump],
    ]];

    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            MintTo {
                mint: ctx.accounts.lp_mint.to_account_info(),
                to: ctx.accounts.user_lp.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
            signer_seeds,
        ),
        lp_to_mint,
    )?;

    Ok(())
}
