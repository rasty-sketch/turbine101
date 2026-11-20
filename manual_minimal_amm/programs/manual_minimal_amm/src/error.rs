use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("Fee must be less than 10,000 bps (100%).")]
    InvalidFee,
    #[msg("Mint X must sort before mint Y.")]
    InvalidMintOrder,
    #[msg("Amount must be greater than zero.")]
    ZeroAmount,
    #[msg("Pool has no liquidity.")]
    EmptyPool,
    #[msg("Slippage tolerance exceeded.")]
    SlippageExceeded,
    #[msg("Arithmetic overflow.")]
    MathOverflow,
    #[msg("The user does not hold enough LP tokens.")]
    InsufficientLp,
}
