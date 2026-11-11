use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    close_account, transfer_checked, CloseAccount, TransferChecked,
};

pub mod types;
pub use types::*;

declare_id!("EzV86xg6N5hMvSogRvaeasKE3VsATy9KEhfKfSoXxyQ");

#[program]
pub mod my_custom_escrow_q4 {
    use super::*;

    pub fn make(
        ctx: Context<Make>,
        amount: u64,
        cost: u64,
        unlock_time: i64,
        deadline: i64,
        escrow_id: u32,
        arbiter: Option<Pubkey>,
    ) -> Result<()> {
        require!(amount > 0, EscrowError::InvalidAmount);
        require!(cost > 0, EscrowError::InvalidCost);

        let clock = Clock::get()?;
        require!(deadline > clock.unix_timestamp, EscrowError::InvalidDeadline);
        require!(
            unlock_time >= 0 && unlock_time < deadline,
            EscrowError::InvalidSchedule
        );
        require_keys_neq!(
            ctx.accounts.offered_mint.key(),
            ctx.accounts.wanted_mint.key(),
            EscrowError::SameMint
        );

        if let Some(arb) = arbiter {
            require!(
                arb != crate::ID && arb != Pubkey::default(),
                EscrowError::InvalidArbiter
            );
        }

        ctx.accounts.escrow.set_inner(Escrow {
            authority: ctx.accounts.user.key(),
            amount,
            cost,
            escrow_id,
            unlock_time,
            deadline,
            offered_mint: ctx.accounts.offered_mint.key(),
            wanted_mint: ctx.accounts.wanted_mint.key(),
            arbiter,
            bump: ctx.bumps.escrow,
        });

        let cpi_ctx = CpiContext::new(
            ctx.accounts.offered_token_program.key(),
            TransferChecked {
                from: ctx.accounts.user_ata.to_account_info(),
                mint: ctx.accounts.offered_mint.to_account_info(),
                to: ctx.accounts.escrow_ata.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        transfer_checked(cpi_ctx, amount, ctx.accounts.offered_mint.decimals)?;

        Ok(())
    }

    pub fn take(ctx: Context<Take>, escrow_id: u32) -> Result<()> {
        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp >= ctx.accounts.escrow.unlock_time,
            EscrowError::EscrowLocked
        );
        require!(
            clock.unix_timestamp <= ctx.accounts.escrow.deadline,
            EscrowError::Expired
        );
        require_keys_neq!(
            ctx.accounts.buyer.key(),
            ctx.accounts.escrow.authority,
            EscrowError::MakerCannotTake
        );

        if let Some(expected_arbiter) = ctx.accounts.escrow.arbiter {
            let arbiter = ctx
                .accounts
                .arbiter
                .as_ref()
                .ok_or(EscrowError::MissingArbiter)?;
            require_keys_eq!(
                arbiter.key(),
                expected_arbiter,
                EscrowError::InvalidArbiter
            );
        }

        let vault_balance = ctx.accounts.escrow_ata.amount;
        require!(
            vault_balance >= ctx.accounts.escrow.amount,
            EscrowError::InsufficientVaultBalance
        );

        let user_key = ctx.accounts.user.key();
        let id_bytes = escrow_id.to_le_bytes();
        let bump = ctx.accounts.escrow.bump;
        let seeds = [ESCROW_SEED, user_key.as_ref(), &id_bytes, &[bump]];
        let signer_seeds = &[&seeds[..]];

        let pay_ctx = CpiContext::new(
            ctx.accounts.wanted_token_program.key(),
            TransferChecked {
                from: ctx.accounts.buyer_wanted_ata.to_account_info(),
                mint: ctx.accounts.wanted_mint.to_account_info(),
                to: ctx.accounts.user_wanted_ata.to_account_info(),
                authority: ctx.accounts.buyer.to_account_info(),
            },
        );
        transfer_checked(
            pay_ctx,
            ctx.accounts.escrow.cost,
            ctx.accounts.wanted_mint.decimals,
        )?;

        let release_ctx = CpiContext::new_with_signer(
            ctx.accounts.offered_token_program.key(),
            TransferChecked {
                from: ctx.accounts.escrow_ata.to_account_info(),
                mint: ctx.accounts.offered_mint.to_account_info(),
                to: ctx.accounts.buyer_ata.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            signer_seeds,
        );
        transfer_checked(
            release_ctx,
            vault_balance,
            ctx.accounts.offered_mint.decimals,
        )?;

        let close_ctx = CpiContext::new_with_signer(
            ctx.accounts.offered_token_program.key(),
            CloseAccount {
                account: ctx.accounts.escrow_ata.to_account_info(),
                destination: ctx.accounts.user.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            signer_seeds,
        );
        close_account(close_ctx)?;

        Ok(())
    }

    pub fn refund(ctx: Context<Refund>, escrow_id: u32) -> Result<()> {
        let user_key = ctx.accounts.user.key();
        let id_bytes = escrow_id.to_le_bytes();
        let bump = ctx.accounts.escrow.bump;
        let seeds = [ESCROW_SEED, user_key.as_ref(), &id_bytes, &[bump]];
        let signer_seeds = &[&seeds[..]];

        let vault_balance = ctx.accounts.escrow_ata.amount;
        if vault_balance > 0 {
            let refund_ctx = CpiContext::new_with_signer(
                ctx.accounts.offered_token_program.key(),
                TransferChecked {
                    from: ctx.accounts.escrow_ata.to_account_info(),
                    mint: ctx.accounts.offered_mint.to_account_info(),
                    to: ctx.accounts.user_ata.to_account_info(),
                    authority: ctx.accounts.escrow.to_account_info(),
                },
                signer_seeds,
            );
            transfer_checked(
                refund_ctx,
                vault_balance,
                ctx.accounts.offered_mint.decimals,
            )?;
        }

        let close_ctx = CpiContext::new_with_signer(
            ctx.accounts.offered_token_program.key(),
            CloseAccount {
                account: ctx.accounts.escrow_ata.to_account_info(),
                destination: ctx.accounts.user.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            signer_seeds,
        );
        close_account(close_ctx)?;

        Ok(())
    }

    pub fn update(
        ctx: Context<Update>,
        escrow_id: u32,
        new_amount: Option<u64>,
        new_cost: Option<u64>,
        new_unlock_time: Option<i64>,
        new_deadline: Option<i64>,
        new_arbiter: Option<Option<Pubkey>>,
    ) -> Result<()> {
        require!(
            new_amount.is_some()
                || new_cost.is_some()
                || new_unlock_time.is_some()
                || new_deadline.is_some()
                || new_arbiter.is_some(),
            EscrowError::NoUpdatesProvided
        );

        let clock = Clock::get()?;
        let final_unlock = new_unlock_time.unwrap_or(ctx.accounts.escrow.unlock_time);
        let final_deadline = new_deadline.unwrap_or(ctx.accounts.escrow.deadline);

        require!(
            final_deadline > clock.unix_timestamp,
            EscrowError::InvalidDeadline
        );
        require!(
            final_unlock >= 0 && final_unlock < final_deadline,
            EscrowError::InvalidSchedule
        );

        if let Some(cost) = new_cost {
            require!(cost > 0, EscrowError::InvalidCost);
            ctx.accounts.escrow.cost = cost;
        }

        ctx.accounts.escrow.unlock_time = final_unlock;
        ctx.accounts.escrow.deadline = final_deadline;

        if let Some(arbiter_opt) = new_arbiter {
            if let Some(arb) = arbiter_opt {
                require!(
                    arb != crate::ID && arb != Pubkey::default(),
                    EscrowError::InvalidArbiter
                );
            }
            ctx.accounts.escrow.arbiter = arbiter_opt;
        }

        if let Some(target_amount) = new_amount {
            require!(target_amount > 0, EscrowError::InvalidAmount);
            let current_amount = ctx.accounts.escrow.amount;
            let decimals = ctx.accounts.offered_mint.decimals;

            if target_amount > current_amount {
                let delta = target_amount
                    .checked_sub(current_amount)
                    .ok_or(EscrowError::MathOverflow)?;
                let deposit_ctx = CpiContext::new(
                    ctx.accounts.offered_token_program.key(),
                    TransferChecked {
                        from: ctx.accounts.user_ata.to_account_info(),
                        mint: ctx.accounts.offered_mint.to_account_info(),
                        to: ctx.accounts.escrow_ata.to_account_info(),
                        authority: ctx.accounts.user.to_account_info(),
                    },
                );
                transfer_checked(deposit_ctx, delta, decimals)?;
            } else if target_amount < current_amount {
                let delta = current_amount
                    .checked_sub(target_amount)
                    .ok_or(EscrowError::MathOverflow)?;
                let user_key = ctx.accounts.user.key();
                let id_bytes = escrow_id.to_le_bytes();
                let bump = ctx.accounts.escrow.bump;
                let seeds = [ESCROW_SEED, user_key.as_ref(), &id_bytes, &[bump]];
                let signer_seeds = &[&seeds[..]];

                let withdraw_ctx = CpiContext::new_with_signer(
                    ctx.accounts.offered_token_program.key(),
                    TransferChecked {
                        from: ctx.accounts.escrow_ata.to_account_info(),
                        mint: ctx.accounts.offered_mint.to_account_info(),
                        to: ctx.accounts.user_ata.to_account_info(),
                        authority: ctx.accounts.escrow.to_account_info(),
                    },
                    signer_seeds,
                );
                transfer_checked(withdraw_ctx, delta, decimals)?;
            }

            ctx.accounts.escrow.amount = target_amount;
        }

        Ok(())
    }
}
