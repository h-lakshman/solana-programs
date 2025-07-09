// notes for below constants
// In Uniswap v3, tick represents log_1.0001(price), so prices change exponentially with ticks.
// But since sol doesn't allow floating-point on-chain, we approximate this with a linear model:
// We take 20000 ticks for 1 unit of sqrt_price.

use crate::error::CLMMError;
use anchor_lang::prelude::*;

pub const BASE_SQRT_PRICE_X64: u128 = 1u128 << 64; // 2^64,Q64.64; base tick repr or tick 0
pub const TICK_PER_BASE: u128 = 20000; // Number of ticks per 2^64 range
pub const TICK_STEP_SIZE: i32 = (BASE_SQRT_PRICE_X64 / TICK_PER_BASE) as i32; // Distance between ticks in sqrt_price_x64 space

pub fn integer_sqrt(value: u128) -> u128 {
    if value == 0 {
        return 0;
    }

    let mut x = value;
    let mut y = (value + 1) / 2;

    while y < x {
        x = y;
        y = (y + value / y) / 2;
    }

    x
}

pub fn tick_to_sqrt_price_x64(tick: i32) -> u128 {
    let tick_adjustment = if tick >= 0 {
        (tick as u128) * (BASE_SQRT_PRICE_X64 / 20000)
    } else {
        ((-tick) as u128) * (BASE_SQRT_PRICE_X64 / 20000)
    };

    if tick >= 0 {
        BASE_SQRT_PRICE_X64 + tick_adjustment
    } else {
        BASE_SQRT_PRICE_X64.saturating_sub(tick_adjustment)
    }
}

pub fn sqrt_price_x64_to_tick(sqrt_price_x64: u128) -> i32 {
    if sqrt_price_x64 >= BASE_SQRT_PRICE_X64 {
        let diff = sqrt_price_x64 - BASE_SQRT_PRICE_X64;
        (diff * 20000 / BASE_SQRT_PRICE_X64) as i32
    } else {
        let diff = BASE_SQRT_PRICE_X64 - sqrt_price_x64;
        -((diff * 20000 / BASE_SQRT_PRICE_X64) as i32)
    }
}

pub fn calculate_liquidity_amounts(
    sqrt_price_current_x64: u128,
    sqrt_price_lower_x64: u128,
    sqrt_price_upper_x64: u128,
    liquidity: u128,
) -> Result<(u64, u64)> {
    let amount_a: u128;
    let amount_b: u128;

    if sqrt_price_current_x64 <= sqrt_price_lower_x64 {
        // Current price is below range, only use token A
        // amount_a = L * (Pu - Pl) / (Pu * Pl)
        let numerator = liquidity
            .checked_mul(sqrt_price_upper_x64 - sqrt_price_lower_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?;

        let denominator = sqrt_price_upper_x64
            .checked_mul(sqrt_price_lower_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?;
        amount_a = mul_div(numerator, BASE_SQRT_PRICE_X64, denominator)?;
        amount_b = 0;
    } else if sqrt_price_current_x64 >= sqrt_price_upper_x64 {
        // Current price is above range, only use token B
        //amount b = L * (Pu -Pl)
        amount_a = 0;
        amount_b = liquidity
            .checked_mul(sqrt_price_upper_x64 - sqrt_price_lower_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?
            .checked_div(BASE_SQRT_PRICE_X64)
            .ok_or(CLMMError::ArithmeticOverflow)?;
    } else {
        // In-range, we need both tokens
        // amount_a = L * (Pu - Pc) / (Pu * Pc)
        // amount_b = L * (Pc - Pl)
        let numerator_a = liquidity
            .checked_mul(sqrt_price_upper_x64 - sqrt_price_current_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?;

        let denominator_a = sqrt_price_upper_x64
            .checked_mul(sqrt_price_current_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?;

        amount_a = mul_div(numerator_a, BASE_SQRT_PRICE_X64, denominator_a)?;

        amount_b = liquidity
            .checked_mul(sqrt_price_current_x64 - sqrt_price_lower_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?
            .checked_div(BASE_SQRT_PRICE_X64)
            .ok_or(CLMMError::ArithmeticOverflow)?;
    }

    Ok((amount_a as u64, amount_b as u64))
}

pub fn compute_swap_step(
    sqrt_price_current_x64: u128,
    sqrt_price_target_x64: u128,
    liquidity: u128,
    amount_remaining: u128,
    a_to_b: bool,
) -> Result<(u128, u128, u128)> {
    let mut amount_in;
    let amount_out;
    let next_sqrt_price_x64;
    //add fee logic later on.

    if a_to_b {
        // moving price down
        // amount_in = L * (Pu - Pl) / (Pu * Pl)
        let numerator1 = liquidity
            .checked_mul(sqrt_price_current_x64 - sqrt_price_target_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?;
        let denominator = sqrt_price_current_x64
            .checked_mul(sqrt_price_target_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?;

        amount_in = mul_div(numerator1, BASE_SQRT_PRICE_X64, denominator)?;
        if amount_remaining >= amount_in {
            // can cross to target
            next_sqrt_price_x64 = sqrt_price_target_x64;
        } else {
            // not enough amount_in to reach target sqrt_price
            // compute new sqrt_price based on available amount_in (amount_remaining)
            // next_sqrt_price = (L * Pc^2) / (L * Pc + Δx * Pc / Q64)

            let numerator = liquidity
                .checked_mul(sqrt_price_current_x64)
                .ok_or(CLMMError::ArithmeticOverflow)?;

            let product = mul_div(
                amount_remaining,
                sqrt_price_current_x64,
                BASE_SQRT_PRICE_X64,
            )?;

            let denominator = numerator
                .checked_add(product)
                .ok_or(CLMMError::ArithmeticOverflow)?;

            next_sqrt_price_x64 = numerator
                .checked_mul(sqrt_price_current_x64)
                .ok_or(CLMMError::ArithmeticOverflow)?
                .checked_div(denominator)
                .ok_or(CLMMError::ArithmeticOverflow)?;
        }
        // amount out = L * (Pu -Pl)
        amount_out = liquidity
            .checked_mul(sqrt_price_current_x64 - next_sqrt_price_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?
            .checked_div(1u128 << 64)
            .ok_or(CLMMError::ArithmeticOverflow)?;
    } else {
        // moving price up
        // amount in = L * (Pu -Pl)
        amount_in = liquidity
            .checked_mul(sqrt_price_target_x64 - sqrt_price_current_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?
            .checked_div(1u128 << 64)
            .ok_or(CLMMError::ArithmeticOverflow)?;

        if amount_remaining >= amount_in {
            next_sqrt_price_x64 = sqrt_price_target_x64;
        } else {
            // next_sqrt_price = sqrt(P) + Δy / L
            let delta = mul_div(amount_remaining, 1u128 << 64, liquidity)?;
            next_sqrt_price_x64 = sqrt_price_current_x64
                .checked_add(delta)
                .ok_or(CLMMError::ArithmeticOverflow)?;
        }

        // amount_out = L * (Pu - Pl) / (Pu * Pl)
        let numerator = liquidity
            .checked_mul(next_sqrt_price_x64 - sqrt_price_current_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?;
        let denominator = sqrt_price_current_x64
            .checked_mul(next_sqrt_price_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?;

        amount_out = mul_div(numerator, 1u128 << 64, denominator)?;
    }

    // recompute amount_in to align with actual next sqrt price
    amount_in = if a_to_b {
        let numerator = liquidity
            .checked_mul(sqrt_price_current_x64 - next_sqrt_price_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?;
        let denominator = sqrt_price_current_x64
            .checked_mul(next_sqrt_price_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?;
        mul_div(numerator, 1u128 << 64, denominator)?
    } else {
        liquidity
            .checked_mul(next_sqrt_price_x64 - sqrt_price_current_x64)
            .ok_or(CLMMError::ArithmeticOverflow)?
            .checked_div(1u128 << 64)
            .ok_or(CLMMError::ArithmeticOverflow)?
    };

    Ok((next_sqrt_price_x64, amount_in, amount_out))
}

pub fn mul_div(a: u128, b: u128, denom: u128) -> Result<u128> {
    Ok(a.checked_mul(b)
        .ok_or(CLMMError::ArithmeticOverflow)?
        .checked_div(denom)
        .ok_or(CLMMError::ArithmeticOverflow)?)
}
