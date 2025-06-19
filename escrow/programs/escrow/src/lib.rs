use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("CG4Ufmj9r1W1YjYRGjUhnx6xqo2AG1UWZte94TstxswS");

#[program]
pub mod escrow_program {
    use super::*;

    pub fn initialize_escrow(ctx: Context<InitializeEscrow>, amount: u64) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow_state;

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.initializer_token_account.to_account_info(),
                to: ctx.accounts.escrow_vault.to_account_info(),
                authority: ctx.accounts.initializer.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, amount)?;

        escrow.initializer = ctx.accounts.initializer.key();
        escrow.vault = ctx.accounts.escrow_vault.key();
        escrow.mint = ctx.accounts.initializer_token_account.mint;
        escrow.amount = amount;
        escrow.bump = ctx.bumps.escrow_state;
        escrow.is_active = true;

        Ok(())
    }

    pub fn exchange(ctx: Context<Exchange>) -> Result<()> {
        let escrow = &ctx.accounts.escrow_state;

        require!(escrow.is_active, EscrowError::EscrowInactive);
        require_eq!(
            ctx.accounts.escrow_vault.amount,
            escrow.amount,
            EscrowError::VaultBalanceMismatch
        );

        let cpi_ctx_1 = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.taker_token_account.to_account_info(),
                to: ctx.accounts.initializer_token_account.to_account_info(),
                authority: ctx.accounts.taker.to_account_info(),
            },
        );
        token::transfer(cpi_ctx_1, escrow.amount)?;

        let bump_seed = [escrow.bump];
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"escrow",
            escrow.initializer.as_ref(),
            escrow.mint.as_ref(),
            &bump_seed,
        ]];
        let cpi_ctx_2 = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.escrow_vault.to_account_info(),
                to: ctx.accounts.taker_token_account.to_account_info(),
                authority: ctx.accounts.escrow_state.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(cpi_ctx_2, escrow.amount)?;

        ctx.accounts.escrow_state.is_active = false;

        Ok(())
    }

    pub fn cancel(ctx: Context<Cancel>) -> Result<()> {
        let escrow = &ctx.accounts.escrow_state;

        require!(escrow.is_active, EscrowError::EscrowInactive);

        let bump_seed = [escrow.bump];
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"escrow",
            escrow.initializer.as_ref(),
            escrow.mint.as_ref(),
            &bump_seed,
        ]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.escrow_vault.to_account_info(),
                to: ctx.accounts.initializer_token_account.to_account_info(),
                authority: ctx.accounts.escrow_state.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(cpi_ctx, escrow.amount)?;

        ctx.accounts.escrow_state.is_active = false;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeEscrow<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,

    #[account(
        mut,
        constraint = initializer_token_account.owner == initializer.key()
    )]
    pub initializer_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = escrow_vault.owner == escrow_state.key(),
        constraint = escrow_vault.mint == initializer_token_account.mint
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = initializer,
        space = 8 + std::mem::size_of::<EscrowState>(),
        seeds = [b"escrow", initializer.key().as_ref(), initializer_token_account.mint.as_ref()],
        bump
    )]
    pub escrow_state: Account<'info, EscrowState>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Exchange<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    #[account(mut, constraint = taker_token_account.owner == taker.key())]
    pub taker_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub initializer_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = escrow_vault.key() == escrow_state.vault,
        constraint = escrow_vault.mint == escrow_state.mint,
        constraint = escrow_vault.owner == escrow_state.key()
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"escrow", escrow_state.initializer.as_ref(), escrow_state.mint.as_ref()],
        bump = escrow_state.bump
    )]
    pub escrow_state: Account<'info, EscrowState>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,

    #[account(
        mut,
        constraint = initializer_token_account.owner == initializer.key()
    )]
    pub initializer_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = escrow_vault.key() == escrow_state.vault,
        constraint = escrow_vault.mint == escrow_state.mint,
        constraint = escrow_vault.owner == escrow_state.key()
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"escrow", escrow_state.initializer.as_ref(), escrow_state.mint.as_ref()],
        bump = escrow_state.bump
    )]
    pub escrow_state: Account<'info, EscrowState>,

    pub token_program: Program<'info, Token>,
}

#[account]
pub struct EscrowState {
    pub initializer: Pubkey,
    pub vault: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub bump: u8,
    pub is_active: bool,
}

#[error_code]
pub enum EscrowError {
    #[msg("Escrow is no longer active.")]
    EscrowInactive,
    #[msg("Vault balance doesn't match escrow amount.")]
    VaultBalanceMismatch,
}
