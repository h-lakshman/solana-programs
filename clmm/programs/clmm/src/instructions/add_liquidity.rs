use crate::{error::CLMMError, state::Pool, utils::price_or_tick_to_sqrt_price_x64};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer,MintTo,mint_to};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

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

    let pool = &mut ctx.accounts.pool;

    require!(
        tick_lower % pool.tick_spacing as i32 == 0 && tick_upper % pool.tick_spacing as i32 == 0,
        CLMMError::UnalignedTick
    );

    let sqrt_price_upper_x64 =
        price_or_tick_to_sqrt_price_x64(Decimal::from_i32(tick_upper).unwrap());
    let sqrt_price_lower_x64 =
        price_or_tick_to_sqrt_price_x64(Decimal::from_i32(tick_lower).unwrap());

    let sqrt_price_current_x64 = if pool.total_lp_issued == 0 {
        let price = Decimal::from_u64(max_quantity_b)
            .unwrap()
            .checked_div(Decimal::from_u64(max_quantity_a).unwrap())
            .unwrap();

        let sqrt_price = price_or_tick_to_sqrt_price_x64(price);
        pool.sqrt_price_x64 = sqrt_price;
        pool.current_tick = 0;
        sqrt_price
    } else {
        pool.sqrt_price_x64
    };

    let sqrt_current = Decimal::from_u128(sqrt_price_current_x64).unwrap();
    let sqrt_upper = Decimal::from_u128(sqrt_price_upper_x64).unwrap();
    let sqrt_lower = Decimal::from_u128(sqrt_price_lower_x64).unwrap();

    let quantity_a = Decimal::from_u64(max_quantity_a).unwrap();
    let quantity_b = Decimal::from_u64(max_quantity_b).unwrap();

    let liquidity: u128;
    let amount_a: u128;
    let amount_b: u128;

    if sqrt_current <= sqrt_lower {
        // in this case current_price is lower than lower tick.so use only token b.
        // L = ΔB / (sqrt(P_upper) - sqrt(P_lower))
        let delta = sqrt_upper - sqrt_lower;
        liquidity = (quantity_b / delta).to_u128().unwrap();

        amount_a = 0;
        amount_b = liquidity * (sqrt_price_upper_x64 - sqrt_price_lower_x64);
    } else if sqrt_current >= sqrt_upper {
        // in this case current_price is greater than upper tick.so use only token a.
        // L = ΔA * sqrt(P_lower) * sqrt(P_upper) / (sqrt(P_upper) - sqrt(P_lower))
        let delta = sqrt_upper - sqrt_lower;
        let numerator = quantity_a * sqrt_lower * sqrt_upper;
        liquidity = (numerator / delta).to_u128().unwrap();

        amount_a = liquidity * (sqrt_price_upper_x64 - sqrt_price_lower_x64) / sqrt_price_upper_x64
            * sqrt_price_lower_x64;
        amount_b = 0;
    } else {
        // in-range, use both tokens, take min(liquidity_a, liquidity_b)
        // L_a = ΔA * sqrt(P_current) * sqrt(P_upper) / (sqrt(P_upper) - sqrt(P_current))
        let liquidity_a = {
            let numerator = quantity_a * sqrt_current * sqrt_upper;
            let denominator = sqrt_upper - sqrt_current;
            (numerator / denominator).to_u128().unwrap()
        };

        // L_b = ΔB / (sqrt(P_current) - sqrt(P_lower))
        let liquidity_b = {
            let denominator = sqrt_current - sqrt_lower;
            (quantity_b / denominator).to_u128().unwrap()
        };

        liquidity = liquidity_a.min(liquidity_b);

        amount_a = liquidity * (sqrt_price_upper_x64 - sqrt_price_current_x64)
            / (sqrt_price_upper_x64 * sqrt_price_current_x64);
        amount_b = liquidity * (sqrt_price_current_x64 - sqrt_price_lower_x64);
    }

    msg!("Calculated Liquidity: {}", liquidity);

    if !(amount_a == 0) {
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.token_account_a.to_account_info(),
                to: ctx.accounts.vault_a.to_account_info(),
                authority: ctx.accounts.liquidity_provider.to_account_info(),
            },
        );

        transfer(cpi_ctx, amount_a.to_u64().unwrap())?;
    }

    if !(amount_b == 0) {
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.token_account_b.to_account_info(),
                to: ctx.accounts.vault_b.to_account_info(),
                authority: ctx.accounts.liquidity_provider.to_account_info(),
            },
        );

        transfer(cpi_ctx, amount_b.to_u64().unwrap())?;
    }

    let mint_a_key = ctx.accounts.token_mint_a.key();
    let mint_b_key = ctx.accounts.token_mint_b.key();
    let authority_bump = [ctx.bumps.authority];
    let seeds: &[&[u8]] = &[b"authority", mint_a_key.as_ref(), mint_b_key.as_ref(), &authority_bump];
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

    mint_to(cpi_ctx, liquidity.to_u64().unwrap())?;

    pool.total_lp_issued = pool
    .total_lp_issued
    .checked_add(liquidity.to_u64().unwrap())
    .ok_or(CLMMError::ArithmeticOverflow)?;

    Ok(())
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
