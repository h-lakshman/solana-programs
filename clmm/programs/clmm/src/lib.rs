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
    pub fn initialize(ctx: Context<InitializePool>, tick_spacing: u16) -> Result<()> {
        instructions::initialize_pool::initialize_pool(ctx, tick_spacing)
    }

    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        max_quantity_a: u64,
        max_quantity_b: u64,
        min_quantity_a: u64,
        min_quantity_b: u64,
        tick_upper: i32,
        tick_lower: i32,
    ) -> Result<()> {
        instructions::add_liquidity(
            ctx,
            max_quantity_a,
            max_quantity_b,
            min_quantity_a,
            min_quantity_b,
            tick_upper,
            tick_lower,
        )
    }

    pub fn withdraw_liquidity(
        ctx: Context<WithdrawLiquidity>,
        lp_tokens_to_withdraw: u64,
        tick_upper: i32,
        tick_lower: i32,
    ) -> Result<()> {
        instructions::withdraw_liquidity(ctx, lp_tokens_to_withdraw, tick_upper, tick_lower)
    }
}
