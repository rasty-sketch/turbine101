use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

pub const ESCROW_SEED: &[u8] = b"escrow";

#[derive(Accounts)]
#[instruction(amount: u64, cost: u64, unlock_time: i64, deadline: i64, escrow_id: u32)]
pub struct Make<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mint::token_program = offered_token_program)]
    pub offered_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mint::token_program = wanted_token_program)]
    pub wanted_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = offered_mint,
        associated_token::authority = user,
        associated_token::token_program = offered_token_program,
    )]
    pub user_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = user,
        seeds = [ESCROW_SEED, user.key().as_ref(), &escrow_id.to_le_bytes()],
        bump,
        space = 8 + Escrow::INIT_SPACE,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = offered_mint,
        associated_token::authority = escrow,
        associated_token::token_program = offered_token_program,
    )]
    pub escrow_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    pub offered_token_program: Interface<'info, TokenInterface>,
    pub wanted_token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(escrow_id: u32)]
pub struct Take<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    /// Optional condition co-signer required when `escrow.arbiter` is `Some(pubkey)`.
    pub arbiter: Option<Signer<'info>>,

    /// CHECK: Validated against `escrow.authority` and PDA seeds; receives closed account rent.
    #[account(
        mut,
        address = escrow.authority @ EscrowError::Unauthorized,
        constraint = user.key() != buyer.key() @ EscrowError::MakerCannotTake,
    )]
    pub user: UncheckedAccount<'info>,

    #[account(
        address = escrow.offered_mint,
        mint::token_program = offered_token_program,
    )]
    pub offered_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        address = escrow.wanted_mint,
        mint::token_program = wanted_token_program,
    )]
    pub wanted_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        close = user,
        seeds = [ESCROW_SEED, user.key().as_ref(), &escrow_id.to_le_bytes()],
        bump = escrow.bump,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    #[account(
        mut,
        associated_token::mint = offered_mint,
        associated_token::authority = escrow,
        associated_token::token_program = offered_token_program,
    )]
    pub escrow_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = offered_mint,
        associated_token::authority = buyer,
        associated_token::token_program = offered_token_program,
    )]
    pub buyer_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = wanted_mint,
        associated_token::authority = buyer,
        associated_token::token_program = wanted_token_program,
    )]
    pub buyer_wanted_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = wanted_mint,
        associated_token::authority = user,
        associated_token::token_program = wanted_token_program,
    )]
    pub user_wanted_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    pub offered_token_program: Interface<'info, TokenInterface>,
    pub wanted_token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(escrow_id: u32)]
pub struct Refund<'info> {
    #[account(
        mut,
        address = escrow.authority @ EscrowError::Unauthorized,
    )]
    pub user: Signer<'info>,

    #[account(
        address = escrow.offered_mint,
        mint::token_program = offered_token_program,
    )]
    pub offered_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        close = user,
        seeds = [ESCROW_SEED, user.key().as_ref(), &escrow_id.to_le_bytes()],
        bump = escrow.bump,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    #[account(
        mut,
        associated_token::mint = offered_mint,
        associated_token::authority = escrow,
        associated_token::token_program = offered_token_program,
    )]
    pub escrow_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = offered_mint,
        associated_token::authority = user,
        associated_token::token_program = offered_token_program,
    )]
    pub user_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    pub offered_token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(escrow_id: u32)]
pub struct Update<'info> {
    #[account(
        mut,
        address = escrow.authority @ EscrowError::Unauthorized,
    )]
    pub user: Signer<'info>,

    #[account(
        address = escrow.offered_mint,
        mint::token_program = offered_token_program,
    )]
    pub offered_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        seeds = [ESCROW_SEED, user.key().as_ref(), &escrow_id.to_le_bytes()],
        bump = escrow.bump,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    #[account(
        mut,
        associated_token::mint = offered_mint,
        associated_token::authority = escrow,
        associated_token::token_program = offered_token_program,
    )]
    pub escrow_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = offered_mint,
        associated_token::authority = user,
        associated_token::token_program = offered_token_program,
    )]
    pub user_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    pub offered_token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace, Debug, PartialEq, Eq)]
pub struct Escrow {
    pub authority: Pubkey,
    pub amount: u64,
    pub cost: u64,
    pub escrow_id: u32,
    pub unlock_time: i64,
    pub deadline: i64,
    pub offered_mint: Pubkey,
    pub wanted_mint: Pubkey,
    pub arbiter: Option<Pubkey>,
    pub bump: u8,
}

#[error_code]
pub enum EscrowError {
    #[msg("Amount must be greater than 0")]
    InvalidAmount,
    #[msg("Cost must be greater than 0")]
    InvalidCost,
    #[msg("Deadline must be in the future")]
    InvalidDeadline,
    #[msg("Unlock time must be non-negative and strictly before deadline")]
    InvalidSchedule,
    #[msg("Escrow has not reached its unlock time yet")]
    EscrowLocked,
    #[msg("Escrow has expired")]
    Expired,
    #[msg("Maker cannot take their own escrow")]
    MakerCannotTake,
    #[msg("Offered mint cannot equal wanted mint")]
    SameMint,
    #[msg("Unauthorized caller")]
    Unauthorized,
    #[msg("Arbiter signature is required to satisfy escrow condition")]
    MissingArbiter,
    #[msg("Invalid arbiter address")]
    InvalidArbiter,
    #[msg("At least one field must be updated")]
    NoUpdatesProvided,
    #[msg("Insufficient vault token balance")]
    InsufficientVaultBalance,
    #[msg("Arithmetic overflow")]
    MathOverflow,
}
