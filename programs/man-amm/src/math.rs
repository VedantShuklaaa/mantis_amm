use anchor_lang::prelude::*;

use crate::{error::AmmError, BPS_DENOMINATOR, MINIMUM_LIQUIDITY};

/// Integer square root, rounded down (Newton's method).
pub fn isqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    if n <= 3 {
        return 1;
    }

    let mut z = n;
    let mut x = n / 2 + 1;
    while x < z {
        z = x;
        x = (n / x + x) / 2;
    }
    z
}

fn to_u64(x: u128) -> Result<u64> {
    u64::try_from(x).map_err(|_| error!(AmmError::MathOverflow))
}

/// Ceiling division, used where rounding must favor the pool.
fn ceil_div(a: u128, b: u128) -> Result<u128> {
    let b_minus_1 = b.checked_sub(1).ok_or(AmmError::MathOverflow)?;
    Ok(a.checked_add(b_minus_1)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(b)
        .ok_or(AmmError::MathOverflow)?)
}

pub fn initial_lp(amount_a: u64, amount_b: u64) -> Result<u64> {
    let product = (amount_a as u128)
        .checked_mul(amount_b as u128)
        .ok_or(AmmError::MathOverflow)?;

    let lp = to_u64(isqrt(product))?;
    require!(lp > MINIMUM_LIQUIDITY, AmmError::InsufficientLiquidity);
    Ok(lp
        .checked_sub(MINIMUM_LIQUIDITY)
        .ok_or(AmmError::MathOverflow)?)
}

/// Later deposits: LP = min(a * supply / reserve_a, b * supply / reserve_b), rounded down.
pub fn lp_for_deposit(
    amount_a: u64,
    amount_b: u64,
    reserve_a: u64,
    reserve_b: u64,
    lp_supply: u64,
) -> Result<u64> {
    require!(
        reserve_a > 0 && reserve_b > 0 && lp_supply > 0,
        AmmError::EmptyPool
    );
    let lp_a = (amount_a as u128)
        .checked_mul(lp_supply as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(reserve_a as u128)
        .ok_or(AmmError::MathOverflow)?;
    let lp_b = (amount_b as u128)
        .checked_mul(lp_supply as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(reserve_b as u128)
        .ok_or(AmmError::MathOverflow)?;
    to_u64(lp_a.min(lp_b))
}

/// Exact (a, b) to pull from the user so the deposit matches the pool ratio.
/// Needed amounts are rounded UP so the pool is never shorted.
pub fn amounts_for_deposit(
    max_a: u64,
    max_b: u64,
    reserve_a: u64,
    reserve_b: u64,
) -> Result<(u64, u64)> {
    require!(reserve_a > 0 && reserve_b > 0, AmmError::EmptyPool);

    let b_needed = ceil_div(
        (max_a as u128)
            .checked_mul(reserve_b as u128)
            .ok_or(AmmError::MathOverflow)?,
        reserve_a as u128,
    )?;
    if b_needed <= max_b as u128 {
        return Ok((max_a, to_u64(b_needed)?));
    }

    let a_needed = ceil_div(
        (max_b as u128)
            .checked_mul(reserve_a as u128)
            .ok_or(AmmError::MathOverflow)?,
        reserve_b as u128,
    )?;
    Ok((to_u64(a_needed)?, max_b))
}

/// Constant-product swap output with the fee taken from the input. Rounded down.
pub fn swap_out(amount_in: u64, reserve_in: u64, reserve_out: u64, fee_bps: u16) -> Result<u64> {
    require!(reserve_in > 0 && reserve_out > 0, AmmError::EmptyPool);

    let fee_factor = BPS_DENOMINATOR
        .checked_sub(fee_bps as u64)
        .ok_or(AmmError::MathOverflow)? as u128;
    let in_after_fee = (amount_in as u128)
        .checked_mul(fee_factor)
        .ok_or(AmmError::MathOverflow)?;

    let numerator = in_after_fee
        .checked_mul(reserve_out as u128)
        .ok_or(AmmError::MathOverflow)?;
    let denominator = (reserve_in as u128)
        .checked_mul(BPS_DENOMINATOR as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_add(in_after_fee)
        .ok_or(AmmError::MathOverflow)?;

    let out = to_u64(
        numerator
            .checked_div(denominator)
            .ok_or(AmmError::MathOverflow)?,
    )?;
    require!(out > 0, AmmError::ZeroOutput);
    Ok(out)
}

/// Pro-rata share of both reserves for burning `lp_amount`. Rounded down.
pub fn withdraw_amounts(
    lp_amount: u64,
    lp_supply: u64,
    reserve_a: u64,
    reserve_b: u64,
) -> Result<(u64, u64)> {
    require!(lp_supply > 0, AmmError::EmptyPool);
    let out_a = (lp_amount as u128)
        .checked_mul(reserve_a as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(lp_supply as u128)
        .ok_or(AmmError::MathOverflow)?;
    let out_b = (lp_amount as u128)
        .checked_mul(reserve_b as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(lp_supply as u128)
        .ok_or(AmmError::MathOverflow)?;
    Ok((to_u64(out_a)?, to_u64(out_b)?))
}
