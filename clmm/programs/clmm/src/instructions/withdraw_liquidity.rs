use anchor_lang::prelude::*;
use anchor_spl::token::{burn, transfer, Burn, Mint, Token, TokenAccount, Transfer};

use crate::{error::CLMMError, state::Pool};

pub fn withdraw_liquidity(
    ctx: Context<WithdrawLiquidity>,
    lp_tokens_to_withdraw: u64,
    tick_upper: i32,
    tick_lower: i32,
) -> Result<()> {
    require!(tick_upper > tick_lower, CLMMError::TickMismatch);

    let pool = &mut ctx.accounts.pool;
    require!(lp_tokens_to_withdraw > 0, CLMMError::ZeroAmount);
    require!(
        ctx.accounts.lp_token_account.amount >= lp_tokens_to_withdraw,
        CLMMError::InsufficientLPTokens
    );
    require!(
        pool.total_lp_issued > 0,
        CLMMError::PoolEmpty
    );
    require!(
        tick_lower % pool.tick_spacing as i32 == 0 && tick_upper % pool.tick_spacing as i32 == 0,
        CLMMError::UnalignedTick
    );

    let vault_a_amount = ctx.accounts.vault_a.amount;
    let vault_b_amount = ctx.accounts.vault_b.amount;
    
    let amount_a = (vault_a_amount as u128)
        .checked_mul(lp_tokens_to_withdraw as u128)
        .ok_or(CLMMError::ArithmeticOverflow)?
        .checked_div(pool.total_lp_issued as u128)
        .ok_or(CLMMError::ArithmeticOverflow)?;

    let amount_b = (vault_b_amount as u128)
        .checked_mul(lp_tokens_to_withdraw as u128)
        .ok_or(CLMMError::ArithmeticOverflow)?
        .checked_div(pool.total_lp_issued as u128)
        .ok_or(CLMMError::ArithmeticOverflow)?;

    require!(
        amount_a <= vault_a_amount as u128,
        CLMMError::InsufficientFundsInPool
    );
    require!(
        amount_b <= vault_b_amount as u128,
        CLMMError::InsufficientFundsInPool
    );

    msg!("Withdrawing LP tokens: {}", lp_tokens_to_withdraw);
    msg!("Amount A to withdraw: {}, Amount B to withdraw: {}", amount_a, amount_b);

    // Setup authority seeds for signing
    let mint_a_key = ctx.accounts.token_mint_a.key();
    let mint_b_key = ctx.accounts.token_mint_b.key();
    let authority_bump = [ctx.bumps.authority];

    let seeds: &[&[u8]] = &[
        b"authority",
        mint_a_key.as_ref(),
        mint_b_key.as_ref(),
        &authority_bump,
    ];
    let signer = &[seeds];

    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Burn {
            mint: ctx.accounts.lp_token_mint.to_account_info(),
            from: ctx.accounts.lp_token_account.to_account_info(),
            authority: ctx.accounts.liquidity_provider.to_account_info(),
        },
    );

    burn(cpi_ctx, lp_tokens_to_withdraw)?;

    if amount_a > 0 {
        let transfer_amount = if amount_a > u64::MAX as u128 {
            return Err(CLMMError::ArithmeticOverflow.into());
        } else {
            amount_a as u64
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_a.to_account_info(),
                to: ctx.accounts.token_account_a.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
            signer,
        );
        transfer(cpi_ctx, transfer_amount)?;
    }

    if amount_b > 0 {
        let transfer_amount = if amount_b > u64::MAX as u128 {
            return Err(CLMMError::ArithmeticOverflow.into());
        } else {
            amount_b as u64
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_b.to_account_info(),
                to: ctx.accounts.token_account_b.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
            signer,
        );
        transfer(cpi_ctx, transfer_amount)?;
    }

    pool.total_lp_issued = pool
        .total_lp_issued
        .checked_sub(lp_tokens_to_withdraw)
        .ok_or(CLMMError::ArithmeticOverflow)?;

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawLiquidity<'info> {
    #[account(mut)]
    pub liquidity_provider: Signer<'info>,

    pub token_mint_a: Account<'info, Mint>,
    pub token_mint_b: Account<'info, Mint>,

    /// CHECK: This holds the complete authority for vault A and B and lp_mint_token
    #[account(seeds = [b"authority", token_mint_a.key().as_ref(), token_mint_b.key().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = token_mint_a,
        associated_token::authority = liquidity_provider
    )]
    pub token_account_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = token_mint_b,
        associated_token::authority = liquidity_provider
    )]
    pub token_account_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"vault_token", token_mint_a.key().as_ref(), token_mint_b.key().as_ref(), b"A"],
        bump,
        token::mint = token_mint_a, 
        token::authority = authority
    )]
    pub vault_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"vault_token", token_mint_a.key().as_ref(), token_mint_b.key().as_ref(), b"B"],
        bump,
        token::mint = token_mint_b,
        token::authority = authority
    )]
    pub vault_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"pool", token_mint_a.key().as_ref(), token_mint_b.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [b"lp_mint", token_mint_a.key().as_ref(), token_mint_b.key().as_ref()],
        bump,
    )]
    pub lp_token_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = lp_token_mint,
        associated_token::authority = liquidity_provider
    )]
    pub lp_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
