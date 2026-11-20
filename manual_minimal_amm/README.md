# Manual Minimal AMM

My Week 3 Anchor AMM for two SPL Token mints, based on my original `minimal_amm`. Liquidity providers deposit the pair and receive LP tokens. Swap pricing is calculated directly in Rust from the two reserve balances.

Each mint pair has one pool PDA derived from `pool`, `mint_x`, and `mint_y`. The pool owns the two reserve accounts and controls the LP mint.

## Instructions

| Instruction | What it does |
| --- | --- |
| `initialize` | Creates the pool, its two token vaults, and the LP mint. Mint X must sort before mint Y by public key. The initializer sets the swap fee in basis points and becomes the treasury owner. |
| `add_liquidity` | Deposits tokens and mints LP shares. The first deposit sets the starting ratio. Later deposits use only the amounts needed at the current ratio. The caller sets a minimum LP amount. |
| `swap` | Exchanges an exact input amount for at least the requested minimum output. The input vault receives the amount after fees. |
| `remove_liquidity` | Burns LP shares and returns a proportional amount from each vault, subject to the caller's minimums. |

The fee is `amount_in * fee_bps / 10_000`, rounded down to a whole base unit. It goes to the treasury owner's associated token account for the input mint. The first swap in each direction creates that account at the trader's expense if needed. For example, at 30 bps, a swap of 10,000,000 base units sends 30,000 to the treasury and 9,970,000 to the pool.

The swap output is `net_in * reserve_out / (reserve_in + net_in)`, rounded down, where `net_in` is the input after the treasury fee.

## Run the tests

Use Rust 1.89.0, Anchor CLI 1.1.2, and the Solana CLI with `cargo build-sbf`. Run these commands from this directory.

```sh
anchor build --ignore-keys
anchor test --skip-build
```

LiteSVM loads the compiled program in memory. The seven tests cover pool setup, LP deposits and withdrawals, both swap directions, treasury balances, and slippage checks.
