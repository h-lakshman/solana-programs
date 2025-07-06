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
    pub fn initialize(ctx: Context<InitializePool>, current_price: u64) -> Result<()> {
        instructions::initialize_pool::initialize_pool(ctx, current_price)
    }

    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        tick_upper: i32,
        tick_lower: i32,
        liquidity: u128,
    ) -> Result<()> {
        instructions::add_liquidity(ctx, tick_upper, tick_lower, liquidity)
    }

    pub fn withdraw_liquidity(
        ctx: Context<WithdrawLiquidity>,
        tick_upper: i32,
        tick_lower: i32,
        liquidity_to_remove: u128,
    ) -> Result<()> {
        instructions::withdraw_liquidity(ctx, tick_upper, tick_lower, liquidity_to_remove)
    }
}
