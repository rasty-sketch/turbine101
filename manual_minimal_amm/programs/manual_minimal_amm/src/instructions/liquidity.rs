use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Burn, Mint, MintTo, Token, TokenAccount, Transfer},
};

use crate::{constants::*, error::AmmError, state::Pool};

#[derive(Accounts)]
pub struct Liquidity<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [POOL_SEED, pool.mint_x.as_ref(), pool.mint_y.as_ref()],
        bump = pool.bump,
        has_one = lp_mint,
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [LP_SEED, pool.key().as_ref()],
        bump = pool.lp_bump,
    )]
    pub lp_mint: Account<'info, Mint>,

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

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = lp_mint,
        associated_token::authority = user,
    )]
    pub user_lp: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_add_liquidity(
    ctx: Context<Liquidity>,
    amount_x_desired: u64,
    amount_y_desired: u64,
    min_lp_out: u64,
) -> Result<()> {
    require!(
        amount_x_desired > 0 && amount_y_desired > 0,
        AmmError::ZeroAmount
    );

    let reserve_x = ctx.accounts.vault_x.amount as u128;
    let reserve_y = ctx.accounts.vault_y.amount as u128;
    let lp_supply = ctx.accounts.lp_mint.supply as u128;

    let (dx, dy, lp_to_mint) = if lp_supply == 0 {
        let product = u128::from(amount_x_desired) * u128::from(amount_y_desired);
        (amount_x_desired, amount_y_desired, product.isqrt() as u64)
    } else {
        require!(reserve_x > 0 && reserve_y > 0, AmmError::EmptyPool);

        let lp_from_x = u128::from(amount_x_desired) * lp_supply / reserve_x;
        let lp_from_y = u128::from(amount_y_desired) * lp_supply / reserve_y;
        let lp = u64::try_from(lp_from_x.min(lp_from_y)).map_err(|_| AmmError::MathOverflow)?;
        let dx = (u128::from(lp) * reserve_x).div_ceil(lp_supply);
        let dy = (u128::from(lp) * reserve_y).div_ceil(lp_supply);

        (dx as u64, dy as u64, lp)
    };

    require!(lp_to_mint > 0, AmmError::ZeroAmount);
    require!(lp_to_mint >= min_lp_out, AmmError::SlippageExceeded);

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.user_x.to_account_info(),
                to: ctx.accounts.vault_x.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        dx,
    )?;

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.user_y.to_account_info(),
                to: ctx.accounts.vault_y.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        dy,
    )?;

    let pool = &ctx.accounts.pool;
    let seeds = &[
        POOL_SEED,
        pool.mint_x.as_ref(),
        pool.mint_y.as_ref(),
        &[pool.bump],
    ];
    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            MintTo {
                mint: ctx.accounts.lp_mint.to_account_info(),
                to: ctx.accounts.user_lp.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            &[seeds],
        ),
        lp_to_mint,
    )?;

    Ok(())
}

pub fn handle_remove_liquidity(
    ctx: Context<Liquidity>,
    lp_amount: u64,
    min_x_out: u64,
    min_y_out: u64,
) -> Result<()> {
    require!(lp_amount > 0, AmmError::ZeroAmount);

    let reserve_x = ctx.accounts.vault_x.amount as u128;
    let reserve_y = ctx.accounts.vault_y.amount as u128;
    let lp_supply = ctx.accounts.lp_mint.supply as u128;
    require!(lp_supply > 0, AmmError::EmptyPool);
    require!(
        lp_amount <= ctx.accounts.user_lp.amount,
        AmmError::InsufficientLp
    );

    let amount_x_out = (u128::from(lp_amount) * reserve_x / lp_supply) as u64;
    let amount_y_out = (u128::from(lp_amount) * reserve_y / lp_supply) as u64;

    require!(
        amount_x_out >= min_x_out && amount_y_out >= min_y_out,
        AmmError::SlippageExceeded
    );

    token::burn(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Burn {
                mint: ctx.accounts.lp_mint.to_account_info(),
                from: ctx.accounts.user_lp.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        lp_amount,
    )?;

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
                from: ctx.accounts.vault_x.to_account_info(),
                to: ctx.accounts.user_x.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            &[seeds],
        ),
        amount_x_out,
    )?;

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.vault_y.to_account_info(),
                to: ctx.accounts.user_y.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            &[seeds],
        ),
        amount_y_out,
    )?;

    Ok(())
}
