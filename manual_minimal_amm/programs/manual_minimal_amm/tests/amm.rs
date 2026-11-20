use anchor_lang::{
    prelude::Pubkey,
    solana_program::{instruction::Instruction, system_program},
    AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::associated_token::{get_associated_token_address as ata, ID as ATA_PROGRAM};
use litesvm::{types::TransactionResult, LiteSVM};
use litesvm_token::{
    get_spl_account,
    spl_token::state::{Account as TokenAccount, Mint},
    CreateAssociatedTokenAccount, CreateMint, MintTo, TOKEN_ID,
};
use manual_minimal_amm::{accounts, error::AmmError, instruction, Pool, ID, LP_SEED, POOL_SEED};
use solana_instruction_error::InstructionError;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_transaction_error::TransactionError;

const STARTING_BALANCE: u64 = 1_000_000_000;

struct TestPool {
    svm: LiteSVM,
    treasury: Keypair,
    user: Keypair,
    mint_x: Pubkey,
    mint_y: Pubkey,
    pool: Pubkey,
    lp_mint: Pubkey,
    vault_x: Pubkey,
    vault_y: Pubkey,
    user_x: Pubkey,
    user_y: Pubkey,
    user_lp: Pubkey,
}

impl TestPool {
    fn new() -> Self {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(
            ID,
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../target/deploy/manual_minimal_amm.so"
            ),
        )
        .unwrap();

        let treasury = Keypair::new();
        let user = Keypair::new();
        for signer in [&treasury, &user] {
            svm.airdrop(&signer.pubkey(), 1_000_000_000).unwrap();
        }

        let mut mints = [
            CreateMint::new(&mut svm, &treasury)
                .decimals(6)
                .send()
                .unwrap(),
            CreateMint::new(&mut svm, &treasury)
                .decimals(6)
                .send()
                .unwrap(),
        ];
        mints.sort_unstable();
        let [mint_x, mint_y] = mints;

        let user_x = fund_token_account(&mut svm, &treasury, &user.pubkey(), &mint_x);
        let user_y = fund_token_account(&mut svm, &treasury, &user.pubkey(), &mint_y);
        let (pool, _) =
            Pubkey::find_program_address(&[POOL_SEED, mint_x.as_ref(), mint_y.as_ref()], &ID);
        let (lp_mint, _) = Pubkey::find_program_address(&[LP_SEED, pool.as_ref()], &ID);

        Self {
            svm,
            treasury,
            user_lp: ata(&user.pubkey(), &lp_mint),
            user,
            mint_x,
            mint_y,
            pool,
            lp_mint,
            vault_x: ata(&pool, &mint_x),
            vault_y: ata(&pool, &mint_y),
            user_x,
            user_y,
        }
    }

    fn initialize(&mut self, fee_bps: u16) -> TransactionResult {
        let ix = amm_ix(
            accounts::Initialize {
                initializer: self.treasury.pubkey(),
                mint_x: self.mint_x,
                mint_y: self.mint_y,
                pool: self.pool,
                lp_mint: self.lp_mint,
                vault_x: self.vault_x,
                vault_y: self.vault_y,
                token_program: TOKEN_ID,
                associated_token_program: ATA_PROGRAM,
                system_program: system_program::ID,
            },
            instruction::Initialize { fee_bps },
        );
        send(&mut self.svm, &self.treasury, ix)
    }

    fn add(&mut self, x: u64, y: u64, min_lp_out: u64) -> TransactionResult {
        let ix = amm_ix(
            self.liquidity_accounts(),
            instruction::AddLiquidity {
                amount_x_desired: x,
                amount_y_desired: y,
                min_lp_out,
            },
        );
        send(&mut self.svm, &self.user, ix)
    }

    fn remove(&mut self, lp_amount: u64, min_x_out: u64, min_y_out: u64) -> TransactionResult {
        let ix = amm_ix(
            self.liquidity_accounts(),
            instruction::RemoveLiquidity {
                lp_amount,
                min_x_out,
                min_y_out,
            },
        );
        send(&mut self.svm, &self.user, ix)
    }

    fn swap_accounts(&self, is_x_to_y: bool) -> accounts::Swap {
        let mint_in = if is_x_to_y { self.mint_x } else { self.mint_y };
        accounts::Swap {
            user: self.user.pubkey(),
            pool: self.pool,
            vault_x: self.vault_x,
            vault_y: self.vault_y,
            user_x: self.user_x,
            user_y: self.user_y,
            mint_in,
            treasury: self.treasury.pubkey(),
            treasury_ata: ata(&self.treasury.pubkey(), &mint_in),
            token_program: TOKEN_ID,
            associated_token_program: ATA_PROGRAM,
            system_program: system_program::ID,
        }
    }

    fn swap(&mut self, is_x_to_y: bool, amount_in: u64, min_amount_out: u64) -> TransactionResult {
        let ix = amm_ix(
            self.swap_accounts(is_x_to_y),
            instruction::Swap {
                is_x_to_y,
                amount_in,
                min_amount_out,
            },
        );
        send(&mut self.svm, &self.user, ix)
    }

    fn liquidity_accounts(&self) -> accounts::Liquidity {
        accounts::Liquidity {
            user: self.user.pubkey(),
            pool: self.pool,
            lp_mint: self.lp_mint,
            vault_x: self.vault_x,
            vault_y: self.vault_y,
            user_x: self.user_x,
            user_y: self.user_y,
            user_lp: self.user_lp,
            token_program: TOKEN_ID,
            associated_token_program: ATA_PROGRAM,
            system_program: system_program::ID,
        }
    }

    fn balance(&self, account: Pubkey) -> u64 {
        get_spl_account::<TokenAccount>(&self.svm, &account)
            .unwrap()
            .amount
    }

    fn reserves(&self) -> (u64, u64) {
        (self.balance(self.vault_x), self.balance(self.vault_y))
    }
}

fn fund_token_account(svm: &mut LiteSVM, payer: &Keypair, owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let account = CreateAssociatedTokenAccount::new(svm, payer, mint)
        .owner(owner)
        .send()
        .unwrap();
    MintTo::new(svm, payer, mint, &account, STARTING_BALANCE)
        .send()
        .unwrap();
    account
}

fn amm_ix(accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
    Instruction::new_with_bytes(ID, &data.data(), accounts.to_account_metas(None))
}

fn send(svm: &mut LiteSVM, payer: &Keypair, ix: Instruction) -> TransactionResult {
    svm.expire_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
}

fn assert_error(result: TransactionResult, expected: impl Into<u32>) {
    let failure = result.unwrap_err();
    assert_eq!(
        failure.err,
        TransactionError::InstructionError(0, InstructionError::Custom(expected.into())),
        "{:?}",
        failure.meta.logs,
    );
}

#[test]
fn initialize_sets_fee_and_treasury() {
    let mut pool = TestPool::new();
    assert_error(pool.initialize(10_000), AmmError::InvalidFee);
    assert!(pool.svm.get_account(&pool.pool).is_none());

    pool.initialize(30).unwrap();
    let account = pool.svm.get_account(&pool.pool).unwrap();
    let state = Pool::try_deserialize(&mut account.data.as_slice()).unwrap();
    assert_eq!((state.mint_x, state.mint_y), (pool.mint_x, pool.mint_y));
    assert_eq!(state.lp_mint, pool.lp_mint);
    assert_eq!(state.fee_bps, 30);
    assert_eq!(state.treasury, pool.treasury.pubkey());
    assert_eq!(pool.reserves(), (0, 0));
}

#[test]
fn add_liquidity_mints_lp_and_uses_only_the_required_tokens() {
    let mut pool = TestPool::new();
    pool.initialize(30).unwrap();

    assert_error(pool.add(100, 225, 151), AmmError::SlippageExceeded);
    assert_eq!(pool.reserves(), (0, 0));

    pool.add(100, 225, 150).unwrap();
    assert_eq!(pool.reserves(), (100, 225));
    assert_eq!(pool.balance(pool.user_lp), 150);

    pool.add(13, 60, 19).unwrap();
    assert_eq!(pool.reserves(), (113, 254));
    assert_eq!(pool.balance(pool.user_lp), 169);

    pool.add(30, 16, 10).unwrap();
    assert_eq!(pool.reserves(), (120, 270));
    assert_eq!(pool.balance(pool.user_lp), 179);
}

#[test]
fn remove_liquidity_burns_lp_and_returns_reserves() {
    let mut pool = TestPool::new();
    pool.initialize(30).unwrap();
    pool.add(100, 225, 150).unwrap();

    assert_error(pool.remove(151, 0, 0), AmmError::InsufficientLp);
    assert_error(pool.remove(75, 50, 113), AmmError::SlippageExceeded);
    assert_eq!(pool.reserves(), (100, 225));
    assert_eq!(pool.balance(pool.user_lp), 150);

    pool.remove(75, 50, 112).unwrap();
    assert_eq!(pool.reserves(), (50, 113));
    assert_eq!(pool.balance(pool.user_lp), 75);
    assert_eq!(pool.balance(pool.user_x), STARTING_BALANCE - 50);
    assert_eq!(pool.balance(pool.user_y), STARTING_BALANCE - 113);

    pool.remove(75, 50, 113).unwrap();
    assert_eq!(pool.reserves(), (0, 0));
    assert_eq!(pool.balance(pool.user_lp), 0);
    assert_eq!(pool.balance(pool.user_x), STARTING_BALANCE);
    assert_eq!(pool.balance(pool.user_y), STARTING_BALANCE);
    assert_eq!(
        get_spl_account::<Mint>(&pool.svm, &pool.lp_mint)
            .unwrap()
            .supply,
        0
    );
}

#[test]
fn swaps_send_fees_to_the_treasury_in_both_directions() {
    let mut pool = TestPool::new();
    pool.initialize(30).unwrap();
    pool.add(100_000_000, 400_000_000, 200_000_000).unwrap();

    let treasury_x = ata(&pool.treasury.pubkey(), &pool.mint_x);
    let treasury_y = ata(&pool.treasury.pubkey(), &pool.mint_y);
    assert!(pool.svm.get_account(&treasury_x).is_none());
    assert!(pool.svm.get_account(&treasury_y).is_none());

    pool.swap(true, 10_000_000, 36_264_435).unwrap();
    assert_eq!(pool.reserves(), (109_970_000, 363_735_565));
    assert_eq!(pool.balance(treasury_x), 30_000);
    assert_eq!(pool.balance(pool.user_x), 890_000_000);
    assert_eq!(pool.balance(pool.user_y), 636_264_435);

    pool.swap(false, 10_000_000, 2_933_862).unwrap();
    assert_eq!(pool.reserves(), (107_036_138, 373_705_565));
    assert_eq!(pool.balance(treasury_x), 30_000);
    assert_eq!(pool.balance(treasury_y), 30_000);
    assert_eq!(pool.balance(pool.user_x), 892_933_862);
    assert_eq!(pool.balance(pool.user_y), 626_264_435);
}

#[test]
fn swap_rejects_empty_pool_slippage_and_wrong_treasury() {
    let mut pool = TestPool::new();
    pool.initialize(30).unwrap();
    assert_error(pool.swap(true, 100, 0), AmmError::EmptyPool);

    pool.add(100_000_000, 400_000_000, 200_000_000).unwrap();
    assert_error(
        pool.swap(true, 10_000_000, 36_264_436),
        AmmError::SlippageExceeded,
    );

    let mut accounts = pool.swap_accounts(true);
    accounts.treasury = Pubkey::new_unique();
    accounts.treasury_ata = ata(&accounts.treasury, &pool.mint_x);
    let ix = amm_ix(
        accounts,
        instruction::Swap {
            is_x_to_y: true,
            amount_in: 10_000_000,
            min_amount_out: 0,
        },
    );
    assert_error(
        send(&mut pool.svm, &pool.user, ix),
        anchor_lang::error::ErrorCode::ConstraintAddress as u32,
    );

    assert_eq!(pool.reserves(), (100_000_000, 400_000_000));
    assert_eq!(pool.balance(pool.user_x), 900_000_000);
    assert!(pool
        .svm
        .get_account(&ata(&pool.treasury.pubkey(), &pool.mint_x))
        .is_none());
}

#[test]
fn treasury_owner_can_swap_using_its_own_token_account() {
    let mut pool = TestPool::new();
    pool.initialize(30).unwrap();
    pool.add(100_000_000, 400_000_000, 200_000_000).unwrap();

    let owner = pool.treasury.pubkey();
    let treasury_x = fund_token_account(&mut pool.svm, &pool.treasury, &owner, &pool.mint_x);
    let treasury_y = fund_token_account(&mut pool.svm, &pool.treasury, &owner, &pool.mint_y);
    let ix = amm_ix(
        accounts::Swap {
            user: owner,
            user_x: treasury_x,
            user_y: treasury_y,
            ..pool.swap_accounts(true)
        },
        instruction::Swap {
            is_x_to_y: true,
            amount_in: 10_000_000,
            min_amount_out: 36_264_435,
        },
    );
    send(&mut pool.svm, &pool.treasury, ix).unwrap();

    assert_eq!(pool.reserves(), (109_970_000, 363_735_565));
    assert_eq!(pool.balance(treasury_x), STARTING_BALANCE - 9_970_000);
    assert_eq!(pool.balance(treasury_y), STARTING_BALANCE + 36_264_435);
}

#[test]
fn zero_fee_swap_rounds_output_down() {
    let mut pool = TestPool::new();
    pool.initialize(0).unwrap();
    pool.add(20, 30, 24).unwrap();

    pool.swap(true, 1, 1).unwrap();
    assert_eq!(pool.reserves(), (21, 29));
    assert_eq!(pool.balance(ata(&pool.treasury.pubkey(), &pool.mint_x)), 0);
}
