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
    // Simplified approximation: sqrt(1.0001^tick) ≈ 1 + tick * 0.00005
    let base_price = 1u128 << 64;
    let tick_adjustment = if tick >= 0 {
        (tick as u128) * (base_price / 20000)
    } else {
        ((-tick) as u128) * (base_price / 20000)
    };

    if tick >= 0 {
        base_price + tick_adjustment
    } else {
        base_price.saturating_sub(tick_adjustment)
    }
}

pub fn sqrt_price_x64_to_tick(sqrt_price_x64: u128) -> i32 {
    let base_price = 1u128 << 64;
    if sqrt_price_x64 >= base_price {
        let diff = sqrt_price_x64 - base_price;
        (diff * 20000 / base_price) as i32
    } else {
        let diff = base_price - sqrt_price_x64;
        -((diff * 20000 / base_price) as i32)
    }
}

// Calculate liquidity for amount  when price is above range
pub fn get_liquidity_for_amount_a(
    sqrt_price_lower_x64: u128,
    sqrt_price_upper_x64: u128,
    amount_a: u64,
) -> u128 {
    let amount = amount_a as u128;
    let price_diff = sqrt_price_upper_x64.saturating_sub(sqrt_price_lower_x64);

    if price_diff == 0 {
        return 0;
    }

    amount * (1u128 << 32) / price_diff
}

// Calculate liquidity for amount  when price is below range
pub fn get_liquidity_for_amount_b(
    sqrt_price_lower_x64: u128,
    sqrt_price_upper_x64: u128,
    amount_b: u64,
) -> u128 {
    let amount = amount_b as u128;
    let price_diff = sqrt_price_upper_x64.saturating_sub(sqrt_price_lower_x64);

    if price_diff == 0 {
        return 0;
    }

    amount * (1u128 << 64) / price_diff
}
