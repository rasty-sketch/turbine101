use {
    anchor_lang::{
        prelude::{Clock, Pubkey},
        solana_program::{instruction::Instruction, program_option::COption, program_pack::Pack, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{
            get_associated_token_address_with_program_id, spl_associated_token_account,
            ID as ASSOCIATED_TOKEN_PROGRAM_ID,
        },
        token::ID as TOKEN_PROGRAM_ID,
        token_2022::{spl_token_2022, ID as TOKEN_2022_PROGRAM_ID},
        token_interface::TokenAccount as AnchorTokenAccount,
    },
    litesvm::{types::FailedTransactionMetadata, LiteSVM},
    my_custom_escrow_q4::{Escrow, ESCROW_SEED},
    solana_account::Account,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

const INITIAL_TIMESTAMP: i64 = 1_700_000_000;
const DECIMALS: u8 = 6;

fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/my_custom_escrow_q4.so"
    ));
    svm.add_program(my_custom_escrow_q4::id(), bytes).unwrap();
    set_timestamp(&mut svm, INITIAL_TIMESTAMP);
    svm
}

fn set_timestamp(svm: &mut LiteSVM, unix_timestamp: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = unix_timestamp;
    svm.set_sysvar::<Clock>(&clock);
}

fn send_tx(svm: &mut LiteSVM, payer: &Keypair, signers: &[&Keypair], ixs: &[Instruction]) {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(ixs, Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    svm.send_transaction(tx).unwrap_or_else(|err| {
        panic!("tx failed: {:?}\nlogs: {:#?}", err.err, err.meta.logs);
    });
}

fn send_tx_err(
    svm: &mut LiteSVM,
    payer: &Keypair,
    signers: &[&Keypair],
    ixs: &[Instruction],
) -> FailedTransactionMetadata {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(ixs, Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    svm.send_transaction(tx)
        .expect_err("expected transaction to fail, but it succeeded")
}

fn assert_log_contains(err: &FailedTransactionMetadata, needle: &str) {
    let joined = err.meta.logs.join("\n");
    assert!(
        joined.contains(needle),
        "expected logs to contain `{needle}`, got:\n{joined}"
    );
}

fn create_funded_user(svm: &mut LiteSVM) -> Keypair {
    let kp = Keypair::new();
    svm.airdrop(&kp.pubkey(), 10_000_000_000).unwrap();
    kp
}

fn create_mint(svm: &mut LiteSVM, authority: &Pubkey, token_program: &Pubkey) -> Pubkey {
    let mint = Pubkey::new_unique();
    let mut mint_bytes = [0u8; spl_token_2022::state::Mint::LEN];
    spl_token_2022::state::Mint::pack(
        spl_token_2022::state::Mint {
            mint_authority: COption::Some(*authority),
            supply: 0,
            decimals: DECIMALS,
            is_initialized: true,
            freeze_authority: COption::None,
        },
        &mut mint_bytes,
    )
    .unwrap();

    svm.set_account(
        mint,
        Account {
            lamports: svm.minimum_balance_for_rent_exemption(mint_bytes.len()),
            data: mint_bytes.to_vec(),
            owner: *token_program,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    mint
}

fn create_and_fund_ata(
    svm: &mut LiteSVM,
    mint_authority: &Keypair,
    owner: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
    amount: u64,
) -> Pubkey {
    let ata = get_associated_token_address_with_program_id(owner, mint, token_program);
    let create_ata_ix =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &mint_authority.pubkey(),
            owner,
            mint,
            token_program,
        );
    let mint_to_ix = spl_token_2022::instruction::mint_to(
        token_program,
        mint,
        &ata,
        &mint_authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();

    send_tx(
        svm,
        mint_authority,
        &[mint_authority],
        &[create_ata_ix, mint_to_ix],
    );
    ata
}

fn token_balance(svm: &LiteSVM, ata: &Pubkey) -> u64 {
    let account = svm.get_account(ata).expect("token account missing");
    let mut data: &[u8] = &account.data;
    AnchorTokenAccount::try_deserialize(&mut data)
        .unwrap()
        .amount
}

fn get_escrow_state(svm: &LiteSVM, escrow: &Pubkey) -> Escrow {
    let account = svm.get_account(escrow).expect("escrow account missing");
    let mut data: &[u8] = &account.data;
    Escrow::try_deserialize(&mut data).unwrap()
}

fn find_escrow_pda(maker: &Pubkey, escrow_id: u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[ESCROW_SEED, maker.as_ref(), &escrow_id.to_le_bytes()],
        &my_custom_escrow_q4::id(),
    )
}

#[allow(clippy::too_many_arguments)]
fn build_make_ix(
    user: Pubkey,
    offered_mint: Pubkey,
    wanted_mint: Pubkey,
    offered_token_program: Pubkey,
    wanted_token_program: Pubkey,
    amount: u64,
    cost: u64,
    unlock_time: i64,
    deadline: i64,
    escrow_id: u32,
    arbiter: Option<Pubkey>,
) -> (Instruction, Pubkey, Pubkey) {
    let (escrow, _) = find_escrow_pda(&user, escrow_id);
    let user_ata =
        get_associated_token_address_with_program_id(&user, &offered_mint, &offered_token_program);
    let escrow_ata = get_associated_token_address_with_program_id(
        &escrow,
        &offered_mint,
        &offered_token_program,
    );

    let ix = Instruction::new_with_bytes(
        my_custom_escrow_q4::id(),
        &my_custom_escrow_q4::instruction::Make {
            amount,
            cost,
            unlock_time,
            deadline,
            escrow_id,
            arbiter,
        }
        .data(),
        my_custom_escrow_q4::accounts::Make {
            user,
            offered_mint,
            wanted_mint,
            user_ata,
            escrow,
            escrow_ata,
            offered_token_program,
            wanted_token_program,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    (ix, escrow, escrow_ata)
}

#[allow(clippy::too_many_arguments)]
fn build_take_ix(
    buyer: Pubkey,
    arbiter: Option<Pubkey>,
    user: Pubkey,
    offered_mint: Pubkey,
    wanted_mint: Pubkey,
    offered_token_program: Pubkey,
    wanted_token_program: Pubkey,
    escrow_id: u32,
) -> Instruction {
    let (escrow, _) = find_escrow_pda(&user, escrow_id);
    let escrow_ata = get_associated_token_address_with_program_id(
        &escrow,
        &offered_mint,
        &offered_token_program,
    );
    let buyer_ata =
        get_associated_token_address_with_program_id(&buyer, &offered_mint, &offered_token_program);
    let buyer_wanted_ata =
        get_associated_token_address_with_program_id(&buyer, &wanted_mint, &wanted_token_program);
    let user_wanted_ata =
        get_associated_token_address_with_program_id(&user, &wanted_mint, &wanted_token_program);

    Instruction::new_with_bytes(
        my_custom_escrow_q4::id(),
        &my_custom_escrow_q4::instruction::Take { escrow_id }.data(),
        my_custom_escrow_q4::accounts::Take {
            buyer,
            arbiter,
            user,
            offered_mint,
            wanted_mint,
            escrow,
            escrow_ata,
            buyer_ata,
            buyer_wanted_ata,
            user_wanted_ata,
            offered_token_program,
            wanted_token_program,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn build_refund_ix(
    signer: Pubkey,
    escrow_owner: Pubkey,
    offered_mint: Pubkey,
    offered_token_program: Pubkey,
    escrow_id: u32,
) -> Instruction {
    let (escrow, _) = find_escrow_pda(&escrow_owner, escrow_id);
    let escrow_ata = get_associated_token_address_with_program_id(
        &escrow,
        &offered_mint,
        &offered_token_program,
    );
    let user_ata = get_associated_token_address_with_program_id(
        &signer,
        &offered_mint,
        &offered_token_program,
    );

    Instruction::new_with_bytes(
        my_custom_escrow_q4::id(),
        &my_custom_escrow_q4::instruction::Refund { escrow_id }.data(),
        my_custom_escrow_q4::accounts::Refund {
            user: signer,
            offered_mint,
            escrow,
            escrow_ata,
            user_ata,
            offered_token_program,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

#[allow(clippy::too_many_arguments)]
fn build_update_ix(
    signer: Pubkey,
    escrow_owner: Pubkey,
    offered_mint: Pubkey,
    offered_token_program: Pubkey,
    escrow_id: u32,
    new_amount: Option<u64>,
    new_cost: Option<u64>,
    new_unlock_time: Option<i64>,
    new_deadline: Option<i64>,
    new_arbiter: Option<Option<Pubkey>>,
) -> Instruction {
    let (escrow, _) = find_escrow_pda(&escrow_owner, escrow_id);
    let escrow_ata = get_associated_token_address_with_program_id(
        &escrow,
        &offered_mint,
        &offered_token_program,
    );
    let user_ata = get_associated_token_address_with_program_id(
        &signer,
        &offered_mint,
        &offered_token_program,
    );

    Instruction::new_with_bytes(
        my_custom_escrow_q4::id(),
        &my_custom_escrow_q4::instruction::Update {
            escrow_id,
            new_amount,
            new_cost,
            new_unlock_time,
            new_deadline,
            new_arbiter,
        }
        .data(),
        my_custom_escrow_q4::accounts::Update {
            user: signer,
            offered_mint,
            escrow,
            escrow_ata,
            user_ata,
            offered_token_program,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

#[test]
fn test_make_and_take_full_lifecycle() {
    let mut svm = setup_svm();
    let mint_admin = create_funded_user(&mut svm);
    let maker = create_funded_user(&mut svm);
    let buyer = create_funded_user(&mut svm);

    let offered_mint = create_mint(&mut svm, &mint_admin.pubkey(), &TOKEN_PROGRAM_ID);
    let wanted_mint = create_mint(&mut svm, &mint_admin.pubkey(), &TOKEN_PROGRAM_ID);

    let maker_offered_ata = create_and_fund_ata(
        &mut svm,
        &mint_admin,
        &maker.pubkey(),
        &offered_mint,
        &TOKEN_PROGRAM_ID,
        500_000_000,
    );
    let buyer_wanted_ata = create_and_fund_ata(
        &mut svm,
        &mint_admin,
        &buyer.pubkey(),
        &wanted_mint,
        &TOKEN_PROGRAM_ID,
        300_000_000,
    );

    let escrow_id = 1;
    let amount = 200_000_000;
    let cost = 120_000_000;
    let unlock_time = INITIAL_TIMESTAMP;
    let deadline = INITIAL_TIMESTAMP + 3_600;

    let (make_ix, escrow, escrow_ata) = build_make_ix(
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        amount,
        cost,
        unlock_time,
        deadline,
        escrow_id,
        None,
    );
    send_tx(&mut svm, &maker, &[&maker], &[make_ix]);

    let state = get_escrow_state(&svm, &escrow);
    assert_eq!(state.authority, maker.pubkey());
    assert_eq!(state.amount, amount);
    assert_eq!(state.cost, cost);
    assert_eq!(state.escrow_id, escrow_id);
    assert_eq!(state.unlock_time, unlock_time);
    assert_eq!(state.deadline, deadline);
    assert_eq!(state.offered_mint, offered_mint);
    assert_eq!(state.wanted_mint, wanted_mint);
    assert_eq!(state.arbiter, None);
    assert_eq!(token_balance(&svm, &maker_offered_ata), 300_000_000);
    assert_eq!(token_balance(&svm, &escrow_ata), amount);

    // Note: maker's wanted ATA was never pre-created; `take` initializes it on demand.
    let take_ix = build_take_ix(
        buyer.pubkey(),
        None,
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        escrow_id,
    );
    send_tx(&mut svm, &buyer, &[&buyer], &[take_ix]);

    let buyer_offered_ata = get_associated_token_address_with_program_id(
        &buyer.pubkey(),
        &offered_mint,
        &TOKEN_PROGRAM_ID,
    );
    let maker_wanted_ata = get_associated_token_address_with_program_id(
        &maker.pubkey(),
        &wanted_mint,
        &TOKEN_PROGRAM_ID,
    );

    assert_eq!(token_balance(&svm, &buyer_offered_ata), amount);
    assert_eq!(token_balance(&svm, &buyer_wanted_ata), 180_000_000);
    assert_eq!(token_balance(&svm, &maker_wanted_ata), cost);
    assert_eq!(svm.get_balance(&escrow).unwrap_or(0), 0);
    assert_eq!(svm.get_balance(&escrow_ata).unwrap_or(0), 0);
}

#[test]
fn test_timed_escrow_clock_unlock_and_deadline_with_update_extension() {
    let mut svm = setup_svm();
    let mint_admin = create_funded_user(&mut svm);
    let maker = create_funded_user(&mut svm);
    let buyer = create_funded_user(&mut svm);

    let offered_mint = create_mint(&mut svm, &mint_admin.pubkey(), &TOKEN_PROGRAM_ID);
    let wanted_mint = create_mint(&mut svm, &mint_admin.pubkey(), &TOKEN_PROGRAM_ID);

    create_and_fund_ata(
        &mut svm,
        &mint_admin,
        &maker.pubkey(),
        &offered_mint,
        &TOKEN_PROGRAM_ID,
        100_000_000,
    );
    create_and_fund_ata(
        &mut svm,
        &mint_admin,
        &buyer.pubkey(),
        &wanted_mint,
        &TOKEN_PROGRAM_ID,
        100_000_000,
    );

    let escrow_id = 7;
    let unlock_time = INITIAL_TIMESTAMP + 1_000;
    let deadline = INITIAL_TIMESTAMP + 2_000;

    let (make_ix, _, _) = build_make_ix(
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        100_000_000,
        50_000_000,
        unlock_time,
        deadline,
        escrow_id,
        None,
    );
    send_tx(&mut svm, &maker, &[&maker], &[make_ix]);

    // 1. Before `unlock_time`: `take` must fail with `EscrowLocked`.
    set_timestamp(&mut svm, INITIAL_TIMESTAMP + 500);
    let early_take_ix = build_take_ix(
        buyer.pubkey(),
        None,
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        escrow_id,
    );
    let err = send_tx_err(&mut svm, &buyer, &[&buyer], &[early_take_ix]);
    assert_log_contains(&err, "EscrowLocked");

    // 2. Past `deadline`: `take` must fail with `Expired`.
    set_timestamp(&mut svm, INITIAL_TIMESTAMP + 2_500);
    svm.expire_blockhash();
    let expired_take_ix = build_take_ix(
        buyer.pubkey(),
        None,
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        escrow_id,
    );
    let err = send_tx_err(&mut svm, &buyer, &[&buyer], &[expired_take_ix]);
    assert_log_contains(&err, "Expired");

    // 3. Maker extends `deadline` via `update`, reviving the escrow window.
    let new_deadline = INITIAL_TIMESTAMP + 5_000;
    let update_ix = build_update_ix(
        maker.pubkey(),
        maker.pubkey(),
        offered_mint,
        TOKEN_PROGRAM_ID,
        escrow_id,
        None,
        None,
        None,
        Some(new_deadline),
        None,
    );
    send_tx(&mut svm, &maker, &[&maker], &[update_ix]);

    // 4. Inside `[unlock_time, new_deadline]`: `take` succeeds.
    set_timestamp(&mut svm, INITIAL_TIMESTAMP + 3_000);
    svm.expire_blockhash();
    let valid_take_ix = build_take_ix(
        buyer.pubkey(),
        None,
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        escrow_id,
    );
    send_tx(&mut svm, &buyer, &[&buyer], &[valid_take_ix]);

    let buyer_offered_ata = get_associated_token_address_with_program_id(
        &buyer.pubkey(),
        &offered_mint,
        &TOKEN_PROGRAM_ID,
    );
    assert_eq!(token_balance(&svm, &buyer_offered_ata), 100_000_000);
}

#[test]
fn test_arbiter_signature_condition() {
    let mut svm = setup_svm();
    let mint_admin = create_funded_user(&mut svm);
    let maker = create_funded_user(&mut svm);
    let buyer = create_funded_user(&mut svm);
    let arbiter = create_funded_user(&mut svm);
    let impostor = create_funded_user(&mut svm);

    let offered_mint = create_mint(&mut svm, &mint_admin.pubkey(), &TOKEN_PROGRAM_ID);
    let wanted_mint = create_mint(&mut svm, &mint_admin.pubkey(), &TOKEN_PROGRAM_ID);

    create_and_fund_ata(
        &mut svm,
        &mint_admin,
        &maker.pubkey(),
        &offered_mint,
        &TOKEN_PROGRAM_ID,
        100_000_000,
    );
    create_and_fund_ata(
        &mut svm,
        &mint_admin,
        &buyer.pubkey(),
        &wanted_mint,
        &TOKEN_PROGRAM_ID,
        100_000_000,
    );

    let escrow_id = 42;
    let (make_ix, _, _) = build_make_ix(
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        100_000_000,
        40_000_000,
        INITIAL_TIMESTAMP + 100,
        INITIAL_TIMESTAMP + 5_000,
        escrow_id,
        Some(arbiter.pubkey()),
    );
    send_tx(&mut svm, &maker, &[&maker], &[make_ix]);

    set_timestamp(&mut svm, INITIAL_TIMESTAMP + 200);

    // 1. Missing arbiter signature fails.
    let no_arbiter_ix = build_take_ix(
        buyer.pubkey(),
        None,
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        escrow_id,
    );
    let err = send_tx_err(&mut svm, &buyer, &[&buyer], &[no_arbiter_ix]);
    assert_log_contains(&err, "MissingArbiter");

    // 2. Wrong arbiter co-signer fails.
    let wrong_arbiter_ix = build_take_ix(
        buyer.pubkey(),
        Some(impostor.pubkey()),
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        escrow_id,
    );
    let err = send_tx_err(&mut svm, &buyer, &[&buyer, &impostor], &[wrong_arbiter_ix]);
    assert_log_contains(&err, "InvalidArbiter");

    // 3. Expected arbiter co-signs -> `take` succeeds.
    let valid_take_ix = build_take_ix(
        buyer.pubkey(),
        Some(arbiter.pubkey()),
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        escrow_id,
    );
    send_tx(&mut svm, &buyer, &[&buyer, &arbiter], &[valid_take_ix]);

    let buyer_offered_ata = get_associated_token_address_with_program_id(
        &buyer.pubkey(),
        &offered_mint,
        &TOKEN_PROGRAM_ID,
    );
    assert_eq!(token_balance(&svm, &buyer_offered_ata), 100_000_000);
}

#[test]
fn test_update_amount_cost_schedule_and_arbiter() {
    let mut svm = setup_svm();
    let mint_admin = create_funded_user(&mut svm);
    let maker = create_funded_user(&mut svm);
    let buyer = create_funded_user(&mut svm);
    let arbiter = create_funded_user(&mut svm);
    let attacker = create_funded_user(&mut svm);

    let offered_mint = create_mint(&mut svm, &mint_admin.pubkey(), &TOKEN_PROGRAM_ID);
    let wanted_mint = create_mint(&mut svm, &mint_admin.pubkey(), &TOKEN_PROGRAM_ID);

    let maker_offered_ata = create_and_fund_ata(
        &mut svm,
        &mint_admin,
        &maker.pubkey(),
        &offered_mint,
        &TOKEN_PROGRAM_ID,
        300_000_000,
    );
    create_and_fund_ata(
        &mut svm,
        &mint_admin,
        &buyer.pubkey(),
        &wanted_mint,
        &TOKEN_PROGRAM_ID,
        200_000_000,
    );

    let escrow_id = 9;
    let (make_ix, escrow, escrow_ata) = build_make_ix(
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        100_000_000,
        50_000_000,
        INITIAL_TIMESTAMP,
        INITIAL_TIMESTAMP + 2_000,
        escrow_id,
        None,
    );
    send_tx(&mut svm, &maker, &[&maker], &[make_ix]);

    // 1. Maker tops up `amount` (100M -> 180M), raises `cost` (50M -> 90M), sets arbiter & schedule.
    let top_up_ix = build_update_ix(
        maker.pubkey(),
        maker.pubkey(),
        offered_mint,
        TOKEN_PROGRAM_ID,
        escrow_id,
        Some(180_000_000),
        Some(90_000_000),
        Some(INITIAL_TIMESTAMP + 100),
        Some(INITIAL_TIMESTAMP + 4_000),
        Some(Some(arbiter.pubkey())),
    );
    send_tx(&mut svm, &maker, &[&maker], &[top_up_ix]);

    let state = get_escrow_state(&svm, &escrow);
    assert_eq!(state.amount, 180_000_000);
    assert_eq!(state.cost, 90_000_000);
    assert_eq!(state.unlock_time, INITIAL_TIMESTAMP + 100);
    assert_eq!(state.deadline, INITIAL_TIMESTAMP + 4_000);
    assert_eq!(state.arbiter, Some(arbiter.pubkey()));
    assert_eq!(token_balance(&svm, &maker_offered_ata), 120_000_000);
    assert_eq!(token_balance(&svm, &escrow_ata), 180_000_000);

    // 2. Maker partially withdraws `amount` (180M -> 140M) and clears arbiter back to `None`.
    let reduce_ix = build_update_ix(
        maker.pubkey(),
        maker.pubkey(),
        offered_mint,
        TOKEN_PROGRAM_ID,
        escrow_id,
        Some(140_000_000),
        None,
        None,
        None,
        Some(None),
    );
    send_tx(&mut svm, &maker, &[&maker], &[reduce_ix]);

    let state = get_escrow_state(&svm, &escrow);
    assert_eq!(state.amount, 140_000_000);
    assert_eq!(state.arbiter, None);
    assert_eq!(token_balance(&svm, &maker_offered_ata), 160_000_000);
    assert_eq!(token_balance(&svm, &escrow_ata), 140_000_000);

    // 3. Unauthorized caller cannot update.
    let unauthorized_ix = build_update_ix(
        attacker.pubkey(),
        maker.pubkey(),
        offered_mint,
        TOKEN_PROGRAM_ID,
        escrow_id,
        None,
        Some(1),
        None,
        None,
        None,
    );
    let _ = send_tx_err(&mut svm, &attacker, &[&attacker], &[unauthorized_ix]);

    // 4. Empty update (`all None`) fails with `NoUpdatesProvided`.
    let empty_update_ix = build_update_ix(
        maker.pubkey(),
        maker.pubkey(),
        offered_mint,
        TOKEN_PROGRAM_ID,
        escrow_id,
        None,
        None,
        None,
        None,
        None,
    );
    let err = send_tx_err(&mut svm, &maker, &[&maker], &[empty_update_ix]);
    assert_log_contains(&err, "NoUpdatesProvided");

    // 5. Buyer takes at the updated terms (140M offered for 90M wanted, no arbiter).
    set_timestamp(&mut svm, INITIAL_TIMESTAMP + 500);
    let take_ix = build_take_ix(
        buyer.pubkey(),
        None,
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        escrow_id,
    );
    send_tx(&mut svm, &buyer, &[&buyer], &[take_ix]);

    let buyer_offered_ata = get_associated_token_address_with_program_id(
        &buyer.pubkey(),
        &offered_mint,
        &TOKEN_PROGRAM_ID,
    );
    let maker_wanted_ata = get_associated_token_address_with_program_id(
        &maker.pubkey(),
        &wanted_mint,
        &TOKEN_PROGRAM_ID,
    );
    assert_eq!(token_balance(&svm, &buyer_offered_ata), 140_000_000);
    assert_eq!(token_balance(&svm, &maker_wanted_ata), 90_000_000);
}

#[test]
fn test_refund_before_or_after_deadline_and_dust_griefing_immunity() {
    let mut svm = setup_svm();
    let mint_admin = create_funded_user(&mut svm);
    let maker = create_funded_user(&mut svm);
    let attacker = create_funded_user(&mut svm);

    let offered_mint = create_mint(&mut svm, &mint_admin.pubkey(), &TOKEN_PROGRAM_ID);
    let wanted_mint = create_mint(&mut svm, &mint_admin.pubkey(), &TOKEN_PROGRAM_ID);

    let maker_offered_ata = create_and_fund_ata(
        &mut svm,
        &mint_admin,
        &maker.pubkey(),
        &offered_mint,
        &TOKEN_PROGRAM_ID,
        200_000_000,
    );
    let attacker_offered_ata = create_and_fund_ata(
        &mut svm,
        &mint_admin,
        &attacker.pubkey(),
        &offered_mint,
        &TOKEN_PROGRAM_ID,
        1_000,
    );

    // Escrow #1: refunded before `unlock_time`, even after attacker sends dust tokens to `escrow_ata`.
    let (make_ix_1, escrow_1, escrow_ata_1) = build_make_ix(
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        100_000_000,
        50_000_000,
        INITIAL_TIMESTAMP + 500,
        INITIAL_TIMESTAMP + 1_000,
        1,
        None,
    );
    send_tx(&mut svm, &maker, &[&maker], &[make_ix_1]);

    // Attacker cannot refund maker's escrow.
    let bad_refund_ix = build_refund_ix(
        attacker.pubkey(),
        maker.pubkey(),
        offered_mint,
        TOKEN_PROGRAM_ID,
        1,
    );
    let _ = send_tx_err(&mut svm, &attacker, &[&attacker], &[bad_refund_ix]);

    // Attacker sends 7 dust base units directly into `escrow_ata_1` to attempt `close_account` griefing.
    let dust_ix = spl_token_2022::instruction::transfer_checked(
        &TOKEN_PROGRAM_ID,
        &attacker_offered_ata,
        &offered_mint,
        &escrow_ata_1,
        &attacker.pubkey(),
        &[],
        7,
        DECIMALS,
    )
    .unwrap();
    send_tx(&mut svm, &attacker, &[&attacker], &[dust_ix]);
    assert_eq!(token_balance(&svm, &escrow_ata_1), 100_000_007);

    // Maker refunds -> sweeps full 100_000_007 balance and closes `escrow_ata_1` + `escrow_1`.
    let refund_ix_1 = build_refund_ix(
        maker.pubkey(),
        maker.pubkey(),
        offered_mint,
        TOKEN_PROGRAM_ID,
        1,
    );
    send_tx(&mut svm, &maker, &[&maker], &[refund_ix_1]);

    assert_eq!(token_balance(&svm, &maker_offered_ata), 200_000_007);
    assert_eq!(svm.get_balance(&escrow_1).unwrap_or(0), 0);
    assert_eq!(svm.get_balance(&escrow_ata_1).unwrap_or(0), 0);

    // Escrow #2: refunded after `deadline` expires.
    let (make_ix_2, escrow_2, escrow_ata_2) = build_make_ix(
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        50_000_000,
        25_000_000,
        INITIAL_TIMESTAMP,
        INITIAL_TIMESTAMP + 100,
        2,
        None,
    );
    send_tx(&mut svm, &maker, &[&maker], &[make_ix_2]);

    set_timestamp(&mut svm, INITIAL_TIMESTAMP + 5_000);
    let refund_ix_2 = build_refund_ix(
        maker.pubkey(),
        maker.pubkey(),
        offered_mint,
        TOKEN_PROGRAM_ID,
        2,
    );
    send_tx(&mut svm, &maker, &[&maker], &[refund_ix_2]);

    assert_eq!(token_balance(&svm, &maker_offered_ata), 200_000_007);
    assert_eq!(svm.get_balance(&escrow_2).unwrap_or(0), 0);
    assert_eq!(svm.get_balance(&escrow_ata_2).unwrap_or(0), 0);
}

#[test]
fn test_cross_standard_swap_spl_token_for_token_2022() {
    let mut svm = setup_svm();
    let mint_admin = create_funded_user(&mut svm);
    let maker = create_funded_user(&mut svm);
    let buyer = create_funded_user(&mut svm);

    // Offered mint on legacy SPL Token; wanted mint on Token-2022.
    let offered_mint = create_mint(&mut svm, &mint_admin.pubkey(), &TOKEN_PROGRAM_ID);
    let wanted_mint = create_mint(&mut svm, &mint_admin.pubkey(), &TOKEN_2022_PROGRAM_ID);

    create_and_fund_ata(
        &mut svm,
        &mint_admin,
        &maker.pubkey(),
        &offered_mint,
        &TOKEN_PROGRAM_ID,
        75_000_000,
    );
    create_and_fund_ata(
        &mut svm,
        &mint_admin,
        &buyer.pubkey(),
        &wanted_mint,
        &TOKEN_2022_PROGRAM_ID,
        95_000_000,
    );

    let escrow_id = 99;
    let (make_ix, _, _) = build_make_ix(
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_2022_PROGRAM_ID,
        75_000_000,
        95_000_000,
        INITIAL_TIMESTAMP,
        INITIAL_TIMESTAMP + 1_000,
        escrow_id,
        None,
    );
    send_tx(&mut svm, &maker, &[&maker], &[make_ix]);

    let take_ix = build_take_ix(
        buyer.pubkey(),
        None,
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_2022_PROGRAM_ID,
        escrow_id,
    );
    send_tx(&mut svm, &buyer, &[&buyer], &[take_ix]);

    let buyer_offered_ata = get_associated_token_address_with_program_id(
        &buyer.pubkey(),
        &offered_mint,
        &TOKEN_PROGRAM_ID,
    );
    let maker_wanted_ata = get_associated_token_address_with_program_id(
        &maker.pubkey(),
        &wanted_mint,
        &TOKEN_2022_PROGRAM_ID,
    );
    assert_eq!(token_balance(&svm, &buyer_offered_ata), 75_000_000);
    assert_eq!(token_balance(&svm, &maker_wanted_ata), 95_000_000);
}

#[test]
fn test_validation_guards() {
    let mut svm = setup_svm();
    let mint_admin = create_funded_user(&mut svm);
    let maker = create_funded_user(&mut svm);

    let offered_mint = create_mint(&mut svm, &mint_admin.pubkey(), &TOKEN_PROGRAM_ID);
    let wanted_mint = create_mint(&mut svm, &mint_admin.pubkey(), &TOKEN_PROGRAM_ID);

    create_and_fund_ata(
        &mut svm,
        &mint_admin,
        &maker.pubkey(),
        &offered_mint,
        &TOKEN_PROGRAM_ID,
        200_000_000,
    );
    create_and_fund_ata(
        &mut svm,
        &mint_admin,
        &maker.pubkey(),
        &wanted_mint,
        &TOKEN_PROGRAM_ID,
        200_000_000,
    );

    // 1. Zero amount rejected.
    let (zero_amt_ix, _, _) = build_make_ix(
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        0,
        50,
        INITIAL_TIMESTAMP,
        INITIAL_TIMESTAMP + 1_000,
        1,
        None,
    );
    let err = send_tx_err(&mut svm, &maker, &[&maker], &[zero_amt_ix]);
    assert_log_contains(&err, "InvalidAmount");

    // 2. Zero cost rejected.
    let (zero_cost_ix, _, _) = build_make_ix(
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        50,
        0,
        INITIAL_TIMESTAMP,
        INITIAL_TIMESTAMP + 1_000,
        1,
        None,
    );
    let err = send_tx_err(&mut svm, &maker, &[&maker], &[zero_cost_ix]);
    assert_log_contains(&err, "InvalidCost");

    // 3. Same mint rejected.
    let (same_mint_ix, _, _) = build_make_ix(
        maker.pubkey(),
        offered_mint,
        offered_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        50,
        50,
        INITIAL_TIMESTAMP,
        INITIAL_TIMESTAMP + 1_000,
        1,
        None,
    );
    let err = send_tx_err(&mut svm, &maker, &[&maker], &[same_mint_ix]);
    assert_log_contains(&err, "SameMint");

    // 4. Invalid schedule (`unlock_time >= deadline`) rejected.
    let (bad_sched_ix, _, _) = build_make_ix(
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        50,
        50,
        INITIAL_TIMESTAMP + 1_000,
        INITIAL_TIMESTAMP + 1_000,
        1,
        None,
    );
    let err = send_tx_err(&mut svm, &maker, &[&maker], &[bad_sched_ix]);
    assert_log_contains(&err, "InvalidSchedule");

    // 5. Maker cannot `take` their own escrow (rejected by Anchor's duplicate-mutable / self-take guard).
    let (valid_make_ix, _, _) = build_make_ix(
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        50,
        50,
        INITIAL_TIMESTAMP,
        INITIAL_TIMESTAMP + 1_000,
        1,
        None,
    );
    send_tx(&mut svm, &maker, &[&maker], &[valid_make_ix]);

    let self_take_ix = build_take_ix(
        maker.pubkey(),
        None,
        maker.pubkey(),
        offered_mint,
        wanted_mint,
        TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        1,
    );
    let err = send_tx_err(&mut svm, &maker, &[&maker], &[self_take_ix]);
    assert_log_contains(&err, "ConstraintDuplicateMutableAccount");
}
