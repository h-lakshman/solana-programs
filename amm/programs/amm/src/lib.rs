use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount, Token};

declare_id!("8igYFZBtEYMLPmeeWNo1aFKwhMQfh7aEBJFVspu4vSff");

#[program]
pub mod amm {
    use super::*;
    
    pub fn initialize_pool(ctx: Context<InitPool>) -> Result<()> {
        require!(
            ctx.accounts.token_a_mint.key() != ctx.accounts.token_b_mint.key(),
            AMMError::SameTokenMint
        );

        let pool = &mut ctx.accounts.amm_pool;
        pool.mint_a = ctx.accounts.token_a_mint.key();
        pool.mint_b = ctx.accounts.token_b_mint.key();
        pool.vault_a = ctx.accounts.vault_a.key();
        pool.vault_b = ctx.accounts.vault_b.key();
        pool.lp_mint = ctx.accounts.lp_token_mint.key();
        pool.total_lp_issued = 0;
        pool.bump = ctx.bumps.amm_pool;
        
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitPool<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    /// CHECK: This holds the complete authority for vault A and B and lp_mint_token
    #[account(seeds = [b"authority", token_a_mint.key().as_ref(), token_b_mint.key().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,
    #[account()]
    pub token_a_mint: Account<'info, Mint>,
    #[account()]
    pub token_b_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = initializer,
        seeds = [b"vault_token", token_a_mint.key().as_ref(), token_b_mint.key().as_ref(), b"A"],
        bump,
        token::mint = token_a_mint, 
        token::authority = authority
    )]
    pub vault_a: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = initializer,
        seeds = [b"vault_token", token_a_mint.key().as_ref(), token_b_mint.key().as_ref(), b"B"],
        bump,
        token::mint = token_b_mint,
        token::authority = authority
    )]
    pub vault_b: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = initializer,
        seeds = [b"lp_mint", token_a_mint.key().as_ref(), token_b_mint.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = authority,
        mint::freeze_authority = authority
    )]
    pub lp_token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = initializer,
        space = 8 + (32 * 5) + 16 + 1,
        seeds = [b"pool",
        token_a_mint.key().as_ref(),
        token_b_mint.key().as_ref()],
        bump)]
    pub amm_pool: Account<'info, AMMPool>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct AMMPool {
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub lp_mint: Pubkey,
    pub total_lp_issued: u128,
    pub bump: u8,
}

#[error_code]
pub enum AMMError {
    #[msg("Token A and Token B must be different.")]
    SameTokenMint,
}