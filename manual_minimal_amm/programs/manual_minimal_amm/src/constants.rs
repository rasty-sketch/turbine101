use anchor_lang::prelude::*;

#[constant]
pub const POOL_SEED: &[u8] = b"pool";

#[constant]
pub const LP_SEED: &[u8] = b"lp";

pub const BPS_DENOMINATOR: u128 = 10_000;
