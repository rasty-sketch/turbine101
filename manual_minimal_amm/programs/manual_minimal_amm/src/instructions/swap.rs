use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

use crate::{constants::*, error::AmmError, state::Pool};

#[derive(Accounts)]
#[instruction(is_x_to_y: bool)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [POOL_SEED, pool.mint_x.as_ref(), pool.mint_y.as_ref()],
        bump = pool.bump,
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        associated_token::mint = pool.mint_x,
        associated_token::authority = pool,
    )]
    pub vault_x: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = pool.mint_y,
        associated_token::authority = pool,
    )]
    pub vault_y: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = pool.mint_x,
        associated_token::authority = user,
    )]
    pub user_x: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = pool.mint_y,
        associated_token::authority = user,
    )]
    pub user_y: Account<'info, TokenAccount>,

    #[account(address = if is_x_to_y { pool.mint_x } else { pool.mint_y })]
    pub mint_in: Account<'info, Mint>,

    /// CHECK: The address must match the treasury set at initialization.
    #[account(address = pool.treasury)]
    pub treasury: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        dup,
        payer = user,
        associated_token::mint = mint_in,
        associated_token::authority = treasury,
    )]
    pub treasury_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_swap(
    ctx: Context<Swap>,
    is_x_to_y: bool,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<()> {
    require!(amount_in > 0, AmmError::ZeroAmount);

    let (user_in, user_out, vault_in, vault_out) = if is_x_to_y {
        (
            &ctx.accounts.user_x,
            &ctx.accounts.user_y,
            &ctx.accounts.vault_x,
            &ctx.accounts.vault_y,
        )
    } else {
        (
            &ctx.accounts.user_y,
            &ctx.accounts.user_x,
            &ctx.accounts.vault_y,
            &ctx.accounts.vault_x,
        )
    };
    let reserve_in = vault_in.amount;
    let reserve_out = vault_out.amount;
    require!(reserve_in > 0 && reserve_out > 0, AmmError::EmptyPool);

    let fee =
        (u128::from(amount_in) * u128::from(ctx.accounts.pool.fee_bps) / BPS_DENOMINATOR) as u64;
    let net_in = amount_in - fee;
    let reserve_in_after = reserve_in
        .checked_add(net_in)
        .ok_or(AmmError::MathOverflow)?;
    let amount_out =
        (u128::from(net_in) * u128::from(reserve_out) / u128::from(reserve_in_after)) as u64;

    require!(amount_out > 0, AmmError::ZeroAmount);
    require!(amount_out >= min_amount_out, AmmError::SlippageExceeded);

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Transfer {
                from: user_in.to_account_info(),
                to: vault_in.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        net_in,
    )?;
    if fee > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: user_in.to_account_info(),
                    to: ctx.accounts.treasury_ata.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            fee,
        )?;
    }

    let pool = &ctx.accounts.pool;
    let seeds = &[
        POOL_SEED,
        pool.mint_x.as_ref(),
        pool.mint_y.as_ref(),
        &[pool.bump],
    ];
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            Transfer {
                from: vault_out.to_account_info(),
                to: user_out.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            &[seeds],
        ),
        amount_out,
    )?;

    Ok(())
}
