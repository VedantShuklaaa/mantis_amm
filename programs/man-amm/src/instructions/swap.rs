use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::{error::AmmError, swap_out, Pool, AUTHORITY_SEED, POOL_SEED};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
		seeds = [POOL_SEED, pool.mint_a.key().as_ref(), pool.mint_b.key().as_ref()],
		bump = pool.pool_bump,
		has_one = vault_a,
		has_one = vault_b,
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

	#[account(
		mut, 
		token::mint = pool.mint_a,
		token::authority = user
	)]
	pub user_token_a: Account<'info, TokenAccount>,

	#[account(
		mut,
		token::mint = pool.mint_b,
		token::authority = user,
	)]
    pub user_token_b: Account<'info, TokenAccount>,

	pub token_program: Program<'info, Token>,
}

pub fn handle_swap(ctx: Context<Swap>, amount_in: u64, min_out: u64, a_to_b: bool) -> Result<()> {
	require!(amount_in > 0, AmmError::ZeroAmount);

	let (vault_in, vault_out, user_in, user_out) = if a_to_b {
		(
			&ctx.accounts.vault_a,
			&ctx.accounts.vault_b,
			&ctx.accounts.user_token_a,
			&ctx.accounts.user_token_b,
		)
	} else { 
		(
			&ctx.accounts.vault_b,
		    &ctx.accounts.vault_a,
		    &ctx.accounts.user_token_b,
		    &ctx.accounts.user_token_a,
	    )
	};

	let reserve_in = vault_in.amount;
	let reserve_out = vault_out.amount;

	let amount_out = swap_out(amount_in, reserve_in, reserve_out, ctx.accounts.pool.fee_bps)?;
	require!(amount_out >= min_out, AmmError::SlippageExceeded);

	transfer(
		CpiContext::new(
			ctx.accounts.token_program.key(), 
			Transfer {
				from: user_in.to_account_info(),
				to: vault_in.to_account_info(),
				authority: ctx.accounts.user.to_account_info(),
			}
		), 
		amount_in,
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
                from: vault_out.to_account_info(),
                to: user_out.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount_out,
    )?;

    Ok(())
}
