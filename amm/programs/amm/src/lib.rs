use anchor_lang::prelude::*;

mod error;
mod instructions;
mod state;
mod utils;

use error::*;
use instructions::*;
use state::*;

declare_id!("8igYFZBtEYMLPmeeWNo1aFKwhMQfh7aEBJFVspu4vSff");

#[program]
pub mod amm {
    use super::*;

    pub fn initialize_pool(ctx: Context<InitPool>) -> Result<()> {
        instructions::initialize_pool::initialize_pool(ctx)
    }

    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        quantity_a: u64,
        quantity_b: u64,
    ) -> Result<()> {
        instructions::add_liquidity::add_liquidity(ctx, quantity_a, quantity_b)
    }
}
