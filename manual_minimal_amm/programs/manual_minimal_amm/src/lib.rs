pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("C3CmmLr5H6tXtSprZRoBSHZf46LEDen1dVDxzy3TKiQY");

#[program]
pub mod manual_minimal_amm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
        crate::instructions::initialize::handle_initialize(ctx, fee_bps)
    }

    pub fn add_liquidity(
        ctx: Context<Liquidity>,
        amount_x_desired: u64,
        amount_y_desired: u64,
        min_lp_out: u64,
    ) -> Result<()> {
        crate::instructions::liquidity::handle_add_liquidity(
            ctx,
            amount_x_desired,
            amount_y_desired,
            min_lp_out,
        )
    }

    pub fn swap(
        ctx: Context<Swap>,
        is_x_to_y: bool,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        crate::instructions::swap::handle_swap(ctx, is_x_to_y, amount_in, min_amount_out)
    }

    pub fn remove_liquidity(
        ctx: Context<Liquidity>,
        lp_amount: u64,
        min_x_out: u64,
        min_y_out: u64,
    ) -> Result<()> {
        crate::instructions::liquidity::handle_remove_liquidity(
            ctx, lp_amount, min_x_out, min_y_out,
        )
    }
}
