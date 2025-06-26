use anchor_lang::prelude::*;
mod error;
mod instructions;
mod state;
mod utils;

use instructions::*;

declare_id!("9sfBz349EJEWpqrtFS7KJsgusGfiQBn5UbEJq58DSXvN");

#[program]
mod clmm {
    use super::*;
    pub fn initialize(
        ctx: Context<InitializePool>,
        sqrt_price_x64: u128,
        current_tick: i32,
        tick_spacing: u16,
    ) -> Result<()> {
        instructions::initialize_pool::initialize_pool(
            ctx,
            sqrt_price_x64,
            current_tick,
            tick_spacing,
        )
    }
}
