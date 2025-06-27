use crate::{error::CLMMError, state::Pool, utils::*};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer};

pub fn add_liquidity(
    ctx: Context<AddLiquidity>,
    max_quantity_a: u64,
    max_quantity_b: u64,
    min_quantity_a: u64,
    min_quantity_b: u64,
    tick_upper: i32,
    tick_lower: i32,
) -> Result<()> {
    require!(
        max_quantity_a >= min_quantity_a,
        CLMMError::QuantityMismatch
    );
    require!(
        max_quantity_b >= min_quantity_b,
        CLMMError::QuantityMismatch
    );
    require!(tick_upper > tick_lower, CLMMError::TickMismatch);
    require!(max_quantity_a > 0 || max_quantity_b > 0, CLMMError::ZeroAmount);

    let pool = &mut ctx.accounts.pool;

    require!(
        tick_lower % pool.tick_spacing as i32 == 0 && tick_upper % pool.tick_spacing as i32 == 0,
        CLMMError::UnalignedTick
    );

    let sqrt_price_upper_x64 = tick_to_sqrt_price_x64(tick_upper);
    let sqrt_price_lower_x64 = tick_to_sqrt_price_x64(tick_lower);

    let sqrt_price_current_x64 = if pool.total_lp_issued == 0 {
        require!(max_quantity_a > 0 && max_quantity_b > 0, CLMMError::ZeroAmount);
        
        let price_ratio = (max_quantity_b as u128).checked_mul(1u128 << 64).unwrap() / (max_quantity_a as u128);
        let sqrt_price = integer_sqrt(price_ratio);
        let current_tick = sqrt_price_x64_to_tick(sqrt_price);
        
        pool.sqrt_price_x64 = sqrt_price;
        pool.current_tick = current_tick;
        sqrt_price
    } else {
        pool.sqrt_price_x64
    };

    let (liquidity, amount_a, amount_b) = calculate_liquidity_amounts(
        sqrt_price_current_x64,
        sqrt_price_lower_x64,
        sqrt_price_upper_x64,
        max_quantity_a,
        max_quantity_b,
    )?;

    require!(amount_a >= min_quantity_a, CLMMError::SlippageExceeded);
    require!(amount_b >= min_quantity_b, CLMMError::SlippageExceeded);

    msg!("Calculated Liquidity: {}", liquidity);
    msg!("Amount A: {}, Amount B: {}", amount_a, amount_b);

    // Transfer tokens to vaults
    if amount_a > 0 {
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.token_account_a.to_account_info(),
                to: ctx.accounts.vault_a.to_account_info(),
                authority: ctx.accounts.liquidity_provider.to_account_info(),
            },
        );
        transfer(cpi_ctx, amount_a)?;
    }

    if amount_b > 0 {
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.token_account_b.to_account_info(),
                to: ctx.accounts.vault_b.to_account_info(),
                authority: ctx.accounts.liquidity_provider.to_account_info(),
            },
        );
        transfer(cpi_ctx, amount_b)?;
    }

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

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.lp_token_mint.to_account_info(),
            to: ctx.accounts.lp_token_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        },
        signer,
    );

    mint_to(cpi_ctx, liquidity)?;

    pool.total_lp_issued = pool
        .total_lp_issued
        .checked_add(liquidity)
        .ok_or(CLMMError::ArithmeticOverflow)?;

    Ok(())
}

fn calculate_liquidity_amounts(
    sqrt_price_current_x64: u128,
    sqrt_price_lower_x64: u128,
    sqrt_price_upper_x64: u128,
    max_quantity_a: u64,
    max_quantity_b: u64,
) -> Result<(u64, u64, u64)> {
    let liquidity: u128;
    let amount_a: u64;
    let amount_b: u64;

    if sqrt_price_current_x64 <= sqrt_price_lower_x64 {
        // Current price is below range, only use token B
        liquidity = get_liquidity_for_amount_b(sqrt_price_lower_x64, sqrt_price_upper_x64, max_quantity_b);
        amount_a = 0;
        amount_b = liquidity
            .checked_mul(sqrt_price_upper_x64.saturating_sub(sqrt_price_lower_x64))
            .unwrap_or(0)
            .checked_div(1u128 << 64)
            .unwrap_or(0) as u64;
    } else if sqrt_price_current_x64 >= sqrt_price_upper_x64 {
        // Current price is above range, only use token A
        liquidity = get_liquidity_for_amount_a(sqrt_price_lower_x64, sqrt_price_upper_x64, max_quantity_a);
        amount_b = 0;
        let numerator = liquidity
            .checked_mul(sqrt_price_upper_x64.saturating_sub(sqrt_price_lower_x64))
            .unwrap_or(0);
        let denominator = sqrt_price_upper_x64
            .checked_mul(sqrt_price_lower_x64)
            .unwrap_or(1)
            .checked_div(1u128 << 64)
            .unwrap_or(1);
        amount_a = numerator.checked_div(denominator).unwrap_or(0) as u64;
    } else {
        // In-range, simplified calculation
        let liquidity_a = max_quantity_a as u128;
        let liquidity_b = max_quantity_b as u128;
        
        liquidity = liquidity_a.min(liquidity_b);
        
        amount_a = (liquidity as u64).min(max_quantity_a);
        amount_b = (liquidity as u64).min(max_quantity_b);
    }

    Ok((liquidity as u64, amount_a, amount_b))
}

#[derive(Accounts)]
pub struct AddLiquidity<'info> {
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
        init_if_needed,
        payer = liquidity_provider,
        associated_token::mint = lp_token_mint,
        associated_token::authority = liquidity_provider
    )]
    pub lp_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
