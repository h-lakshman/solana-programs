use anchor_lang::prelude::*;

declare_id!("AFYuoTEtAiRQ3kDgwjPJQFZQrbNnmZ2FFGkq7Wm7piFo");

#[program]
pub mod staking {
    use super::*;

    pub fn create_stake_account(ctx: Context<CreateStakeAccount>) -> Result<()> {
        let user_new_stake_acc = &mut ctx.accounts.user_stake_account;
        let clock = Clock::get()?;
        user_new_stake_acc.owner = ctx.accounts.user.key();
        user_new_stake_acc.staked_amount = 0;
        user_new_stake_acc.total_points = 0;
        user_new_stake_acc.stake_timestamp = clock.unix_timestamp;
        user_new_stake_acc.bump = ctx.bumps.user_stake_account;
        msg!("User Stake account created succesfully");
        Ok(())
    }

    pub fn stake(ctx: Context<Stake>, quantity: u64) -> Result<()> {
        let user_stake_account = &mut ctx.accounts.user_stake_account;
        let clock = Clock::get()?;

        user_stake_account.staked_amount += quantity;
        user_stake_account.stake_timestamp = clock.unix_timestamp;
        
        let transfer_instruction = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.user_stake_account.key(),
            quantity,
        );

        let account_infos = [
            ctx.accounts.user.to_account_info(),
            ctx.accounts.user_stake_account.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ];

        anchor_lang::solana_program::program::invoke(&transfer_instruction, &account_infos)?;

        msg!("{} lamports staked successfully", quantity);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateStakeAccount<'info> {
    #[account(mut)]
    user: Signer<'info>,
    #[account(init, payer = user, space = 8 + 32 + 8 + 8 + 8 + 1, seeds = [b"staked_account", user.key().as_ref()], bump)]
    user_stake_account: Account<'info, StakeAccount>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    user: Signer<'info>,
    #[account(mut, seeds = [b"staked_account", user.key().as_ref()], bump = user_stake_account.bump)]
    user_stake_account: Account<'info, StakeAccount>,
    system_program: Program<'info, System>,
}

#[account]
pub struct StakeAccount {
    pub owner: Pubkey,
    pub staked_amount: u64,
    pub total_points: u64,
    pub stake_timestamp: i64,
    pub bump: u8,
}
