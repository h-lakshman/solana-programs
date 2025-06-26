use anchor_lang::prelude::*;

#[account]
pub struct Pool {
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_authority: Pubkey,
    pub sqrt_price_x64: u128,
    pub total_lp_issued: u64,
    pub current_tick: i32,
    pub tick_spacing: u16,
    pub bump: u8,
}
