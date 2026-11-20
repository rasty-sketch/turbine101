use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{constants::*, error::AmmError, state::Pool};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,

    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    #[account(
        init,
        payer = initializer,
        space = 8 + Pool::INIT_SPACE,
        seeds = [POOL_SEED, mint_x.key().as_ref(), mint_y.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        init,
        payer = initializer,
        seeds = [LP_SEED, pool.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = pool,
    )]
    pub lp_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_x,
        associated_token::authority = pool,
    )]
    pub vault_x: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_y,
        associated_token::authority = pool,
    )]
    pub vault_y: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
    require!(u128::from(fee_bps) < BPS_DENOMINATOR, AmmError::InvalidFee);
    require!(
        ctx.accounts.mint_x.key() < ctx.accounts.mint_y.key(),
        AmmError::InvalidMintOrder
    );

    let pool = &mut ctx.accounts.pool;
    pool.mint_x = ctx.accounts.mint_x.key();
    pool.mint_y = ctx.accounts.mint_y.key();
    pool.lp_mint = ctx.accounts.lp_mint.key();
    pool.fee_bps = fee_bps;
    pool.bump = ctx.bumps.pool;
    pool.lp_bump = ctx.bumps.lp_mint;
    pool.treasury = ctx.accounts.initializer.key();

    Ok(())
}
