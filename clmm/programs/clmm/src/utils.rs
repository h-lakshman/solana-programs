use crate::error::CLMMError;
use anchor_lang::prelude::*;
use rust_decimal::prelude::*; // Only needed decimal crate

const Q64: u128 = 1 << 64;
pub const TICK_SPACING: i32 = 10;

// sqrt_price = sqrt(1.0001 ^ tick)
pub fn tick_to_sqrt_price_x64(tick: i32) -> u128 {
    let price = Decimal::from_f64((1.0001_f64).powi(tick)).unwrap();
    let sqrt_price = price.sqrt().unwrap();
    let sqrt_price_x64 = (sqrt_price * Decimal::from(Q64)).to_u128().unwrap();
    sqrt_price_x64
}

// tick = log(price) / log(1.0001)
pub fn sqrt_price_x64_to_tick(sqrt_price_x64: u128) -> i32 {
    let sqrt_price = Decimal::from(sqrt_price_x64) / Decimal::from(Q64);
    let price = sqrt_price * sqrt_price;
    let log = price.ln() / Decimal::from_f64(1.0001_f64).unwrap().ln();
    log.floor().to_i32().unwrap()
}

pub fn price_to_sqrt_price_x64(price: Decimal) -> Result<u128> {
    if price <= Decimal::ZERO {
        return Err(CLMMError::ZeroAmount)?;
    }

    let sqrt_price = price.sqrt().ok_or(CLMMError::ArithmeticOverflow)?;
    let sqrt_price_x64 = (sqrt_price * Decimal::from(Q64))
        .to_u128()
        .ok_or(CLMMError::ArithmeticOverflow)?;

    Ok(sqrt_price_x64)
}

pub fn calculate_liquidity_amounts(
    sqrt_price_current_x64: u128,
    sqrt_price_lower_x64: u128,
    sqrt_price_upper_x64: u128,
    liquidity: u128,
) -> Result<(u64, u64)> {
    let current = Decimal::from(sqrt_price_current_x64);
    let lower = Decimal::from(sqrt_price_lower_x64);
    let upper = Decimal::from(sqrt_price_upper_x64);
    let l = Decimal::from(liquidity);
    let q64 = Decimal::from(Q64);

    let amount_a: Decimal;
    let amount_b: Decimal;
    if current <= lower {
        // Token A only: amount_a = L * (upper - lower) * Q64 / (upper * lower)
        amount_a = (l * (upper - lower) * q64) / (upper * lower);
        amount_b = Decimal::ZERO;
    } else if current >= upper {
        // Token B only: amount_b = L * (upper - lower) / Q64
        amount_a = Decimal::ZERO;
        amount_b = (l * (upper - lower)) / q64;
    } else {
        // Both tokens:
        // amount_a = L * (upper - current) * Q64 / (upper * current)
        // amount_b = L * (current - lower) / Q64
        amount_a = (l * (upper - current) * q64) / (upper * current);
        amount_b = (l * (current - lower)) / q64;
    }

    Ok((
        amount_a.to_u64().ok_or(CLMMError::ArithmeticOverflow)?,
        amount_b.to_u64().ok_or(CLMMError::ArithmeticOverflow)?,
    ))
}

pub fn compute_swap_step(
    sqrt_price_current_x64: u128,
    sqrt_price_target_x64: u128,
    liquidity: u128,
    amount_remaining: u128,
    a_to_b: bool,
) -> Result<(u128, u128, u128)> {
    let current = Decimal::from(sqrt_price_current_x64);
    let target = Decimal::from(sqrt_price_target_x64);
    let l = Decimal::from(liquidity);
    let delta = Decimal::from(amount_remaining);
    let q64 = Decimal::from(Q64);

    let next_price;
    let amount_in;
    let amount_out;

    if a_to_b {
        // Δx = L * (Pc - Pt) * Q64 / (Pc * Pt)
        let required_in = (l * (current - target) * q64) / (current * target);

        if delta >= required_in {
            // Full fill
            next_price = target;
            amount_in = required_in;
        } else {
            // Partial fill: Pn = (L * Pc^2) / (L * Pc + Δx * Pc / Q64)
            let num = l * current * current;
            let denom = l * current + delta * current / q64;
            next_price = num / denom;
            amount_in = delta;
        }

        // Output: Δy = L * (Pc - Pn) / Q64
        amount_out = (l * (current - next_price)) / q64;
    } else {
        // Required input: Δy = L * (Pt - Pc) / Q64
        let required_in = (l * (target - current)) / q64;

        if delta >= required_in {
            // Full step
            next_price = target;
            amount_in = required_in;
        } else {
            // Partial step: Pn = Pc + Δy / L
            next_price = current + (delta * q64) / l;
            amount_in = delta;
        }

        // Output: Δx = L * (Pn - Pc) * Q64 / (Pc * Pn)
        amount_out = (l * (next_price - current) * q64) / (current * next_price);
    }

    Ok((
        next_price.to_u128().ok_or(CLMMError::ArithmeticOverflow)?,
        amount_in.to_u128().ok_or(CLMMError::ArithmeticOverflow)?,
        amount_out.to_u128().ok_or(CLMMError::ArithmeticOverflow)?,
    ))
}
