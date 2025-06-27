use rust_decimal::prelude::*;

pub fn price_or_tick_to_sqrt_price_x64(n: Decimal) -> u128 {
    let sqrt_price = n.sqrt().unwrap();
    let scale = Decimal::from_u128(1u128 << 64).unwrap(); // 2^64
    let sqrt_price_x64 = sqrt_price * scale;

    sqrt_price_x64.to_u128().unwrap()
}
