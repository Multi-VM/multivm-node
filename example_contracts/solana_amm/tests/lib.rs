use solana_program::{native_token::LAMPORTS_PER_SOL, system_instruction, system_program};
use solana_program_test::*;
use solana_sdk::{
    account::{Account, ReadableAccount},
    hash::Hash,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

struct TestUser {
    keypair: Keypair,
    main_x: Pubkey,
    main_y: Pubkey,
}

impl TestUser {
    async fn new(
        keypair: Keypair,
        banks_client: &mut BanksClient,
        recent_blockhash: Hash,
        x_mint: Pubkey,
        y_mint: Pubkey,
    ) -> Self {
        let main_y = create_token_account(&keypair, &y_mint, banks_client, recent_blockhash).await;

        let main_x = create_token_account(&keypair, &x_mint, banks_client, recent_blockhash).await;

        Self {
            keypair,
            main_x,
            main_y,
        }
    }

    async fn temp_acc(
        &self,
        banks_client: &mut BanksClient,
        recent_blockhash: Hash,
        mint: &Pubkey,
        is_x: bool,
        amount: u64,
    ) -> Pubkey {
        let temp_acc =
            create_token_account(&self.keypair, mint, banks_client, recent_blockhash).await;

        transfer_tokens(
            &self.keypair,
            banks_client,
            recent_blockhash,
            if is_x { &self.main_x } else { &self.main_y },
            &temp_acc,
            amount,
        )
        .await;

        temp_acc
    }
}

const X_DECIMALS: u32 = 8;
const Y_DECIMALS: u32 = 4;

const ONE_X: u64 = 10u64.pow(X_DECIMALS);
const ONE_Y: u64 = 10u64.pow(Y_DECIMALS);

#[tokio::test]
async fn test_amm() {
    let program_id = Pubkey::new_unique();

    let mut program_test = ProgramTest::new(
        "solana_amm",
        program_id,
        processor!(solana_amm::entrypoint::process_instruction),
    );

    let alice_keypair = Keypair::new();
    let bob_keypair = Keypair::new();

    program_test.add_account(
        alice_keypair.pubkey(),
        Account {
            lamports: 100 * LAMPORTS_PER_SOL,
            data: vec![],
            owner: system_program::ID,
            ..Account::default()
        },
    );

    program_test.add_account(
        bob_keypair.pubkey(),
        Account {
            lamports: 100 * LAMPORTS_PER_SOL,
            data: vec![],
            owner: system_program::ID,
            ..Account::default()
        },
    );

    // AMM initialization
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    let (x_mint, x_acc) = mint_token(
        &payer,
        &mut banks_client,
        recent_blockhash,
        X_DECIMALS as u8,
    )
    .await;
    let (y_mint, y_acc) = mint_token(
        &payer,
        &mut banks_client,
        recent_blockhash,
        Y_DECIMALS as u8,
    )
    .await;

    let temp_x_address =
        create_token_account(&payer, &x_mint, &mut banks_client, recent_blockhash).await;
    transfer_tokens(
        &payer,
        &mut banks_client,
        recent_blockhash,
        &x_acc,
        &temp_x_address,
        100 * ONE_X,
    )
    .await;

    let temp_y_address =
        create_token_account(&payer, &y_mint, &mut banks_client, recent_blockhash).await;
    transfer_tokens(
        &payer,
        &mut banks_client,
        recent_blockhash,
        &y_acc,
        &temp_y_address,
        10_000 * ONE_Y,
    )
    .await;

    let init_instruction = solana_amm::instruction::initialize_amm(
        &payer.pubkey(),
        &temp_x_address,
        &x_mint,
        &temp_y_address,
        &y_mint,
        &program_id,
        &spl_token::id(),
    );

    let mut transaction = Transaction::new_with_payer(&[init_instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let (amm_data_account, _) = Pubkey::find_program_address(&[b"data"], &program_id);
    let amm_data = solana_amm::state::AMM::unpack(
        banks_client
            .get_account(amm_data_account)
            .await
            .unwrap()
            .unwrap()
            .data(),
    )
    .unwrap();

    assert_eq!(amm_data.x_amount, 100 * ONE_X);
    assert_eq!(amm_data.y_amount, 10_000 * ONE_Y);
    assert_eq!(amm_data.x_mint, x_mint);
    assert_eq!(amm_data.y_mint, y_mint);

    let (x_vault_address, _) = Pubkey::find_program_address(&[b"x_vault"], &program_id);

    let x_vault_data = spl_token::state::Account::unpack(
        banks_client
            .get_account(x_vault_address)
            .await
            .unwrap()
            .unwrap()
            .data(),
    )
    .unwrap();

    assert_eq!(x_vault_data.amount, 100 * ONE_X);

    let (y_vault_address, _) = Pubkey::find_program_address(&[b"y_vault"], &program_id);

    let y_vault_data = spl_token::state::Account::unpack(
        banks_client
            .get_account(y_vault_address)
            .await
            .unwrap()
            .unwrap()
            .data(),
    )
    .unwrap();
    assert_eq!(y_vault_data.amount, 10_000 * ONE_Y);

    // Preparing test users
    let alice = {
        let alice = TestUser::new(
            alice_keypair,
            &mut banks_client,
            recent_blockhash,
            x_mint,
            y_mint,
        )
        .await;

        transfer_tokens(
            &payer,
            &mut banks_client,
            recent_blockhash,
            &y_acc,
            &alice.main_y,
            100 * ONE_Y,
        )
        .await;

        alice
    };

    let bob = {
        let bob = TestUser::new(
            bob_keypair,
            &mut banks_client,
            recent_blockhash,
            x_mint,
            y_mint,
        )
        .await;

        transfer_tokens(
            &payer,
            &mut banks_client,
            recent_blockhash,
            &y_acc,
            &bob.main_y,
            100 * ONE_Y,
        )
        .await;

        bob
    };

    let x_vault_balance = balance_of(x_vault_address, &mut banks_client).await;
    let y_vault_balance = balance_of(y_vault_address, &mut banks_client).await;
    println!("===================================================");
    println!("X vault balance: {}", x_vault_balance as f64 / ONE_X as f64);
    println!("Y vault balance: {}", y_vault_balance as f64 / ONE_Y as f64);
    println!("===================================================");

    // Alice sending 100 Y to AMM
    {
        let alice_x_balance = balance_of(alice.main_x, &mut banks_client).await;
        let alice_y_balance = balance_of(alice.main_y, &mut banks_client).await;
        println!("===================================================");
        println!("Alice X balance: {}", alice_x_balance as f64 / ONE_X as f64);
        println!("Alice Y balance: {}", alice_y_balance as f64 / ONE_Y as f64);
        println!("===================================================");

        let alice_temp_y = alice
            .temp_acc(
                &mut banks_client,
                recent_blockhash,
                &y_mint,
                false,
                100 * ONE_Y,
            )
            .await;

        let exchange_instruction = solana_amm::instruction::exchange(
            &alice.keypair.pubkey(),
            &alice_temp_y,
            &alice.main_x,
            &spl_token::id(),
            &program_id,
        );

        let mut transaction =
            Transaction::new_with_payer(&[exchange_instruction], Some(&alice.keypair.pubkey()));
        transaction.sign(&[&alice.keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        let alice_x_balance = balance_of(alice.main_x, &mut banks_client).await;
        let alice_y_balance = balance_of(alice.main_y, &mut banks_client).await;
        println!("===================================================");
        println!("Alice X balance: {}", alice_x_balance as f64 / ONE_X as f64);
        println!("Alice Y balance: {}", alice_y_balance as f64 / ONE_Y as f64);
        println!("===================================================");
    }

    let x_vault_balance = balance_of(x_vault_address, &mut banks_client).await;
    let y_vault_balance = balance_of(y_vault_address, &mut banks_client).await;
    println!("===================================================");
    println!("X vault balance: {}", x_vault_balance as f64 / ONE_X as f64);
    println!("Y vault balance: {}", y_vault_balance as f64 / ONE_Y as f64);
    println!("===================================================");

    // Bob sending 100 Y to AMM
    {
        let bob_x_balance = balance_of(bob.main_x, &mut banks_client).await;
        let bob_y_balance = balance_of(bob.main_y, &mut banks_client).await;
        println!("===================================================");
        println!("bob X balance: {}", bob_x_balance as f64 / ONE_X as f64);
        println!("bob Y balance: {}", bob_y_balance as f64 / ONE_Y as f64);
        println!("===================================================");

        let bob_temp_y = bob
            .temp_acc(
                &mut banks_client,
                recent_blockhash,
                &y_mint,
                false,
                100 * ONE_Y,
            )
            .await;

        let exchange_instruction = solana_amm::instruction::exchange(
            &bob.keypair.pubkey(),
            &bob_temp_y,
            &bob.main_x,
            &spl_token::id(),
            &program_id,
        );

        let mut transaction =
            Transaction::new_with_payer(&[exchange_instruction], Some(&bob.keypair.pubkey()));
        transaction.sign(&[&bob.keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        let bob_x_balance = balance_of(bob.main_x, &mut banks_client).await;
        let bob_y_balance = balance_of(bob.main_y, &mut banks_client).await;
        println!("===================================================");
        println!("Bob X balance: {}", bob_x_balance as f64 / ONE_X as f64);
        println!("Bob Y balance: {}", bob_y_balance as f64 / ONE_Y as f64);
        println!("===================================================");
    }

    let x_vault_balance = balance_of(x_vault_address, &mut banks_client).await;
    let y_vault_balance = balance_of(y_vault_address, &mut banks_client).await;
    println!("===================================================");
    println!("X vault balance: {}", x_vault_balance as f64 / ONE_X as f64);
    println!("Y vault balance: {}", y_vault_balance as f64 / ONE_Y as f64);
    println!("===================================================");

    // Alice sending all X to AMM
    {
        let alice_x_balance = balance_of(alice.main_x, &mut banks_client).await;

        let alice_temp_x = alice
            .temp_acc(
                &mut banks_client,
                recent_blockhash,
                &x_mint,
                true,
                alice_x_balance,
            )
            .await;

        let exchange_instruction = solana_amm::instruction::exchange(
            &alice.keypair.pubkey(),
            &alice_temp_x,
            &alice.main_y,
            &spl_token::id(),
            &program_id,
        );

        let mut transaction =
            Transaction::new_with_payer(&[exchange_instruction], Some(&alice.keypair.pubkey()));
        transaction.sign(&[&alice.keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        let alice_x_balance = balance_of(alice.main_x, &mut banks_client).await;
        let alice_y_balance = balance_of(alice.main_y, &mut banks_client).await;
        println!("===================================================");
        println!("Alice X balance: {}", alice_x_balance as f64 / ONE_X as f64);
        println!("Alice Y balance: {}", alice_y_balance as f64 / ONE_Y as f64);
        println!("===================================================");
    }

    let x_vault_balance = balance_of(x_vault_address, &mut banks_client).await;
    let y_vault_balance = balance_of(y_vault_address, &mut banks_client).await;
    println!("===================================================");
    println!("X vault balance: {}", x_vault_balance as f64 / ONE_X as f64);
    println!("Y vault balance: {}", y_vault_balance as f64 / ONE_Y as f64);
    println!("===================================================");

    // Bob sending all X to AMM
    {
        let bob_x_balance = balance_of(bob.main_x, &mut banks_client).await;

        let bob_temp_x = bob
            .temp_acc(
                &mut banks_client,
                recent_blockhash,
                &x_mint,
                true,
                bob_x_balance,
            )
            .await;

        let exchange_instruction = solana_amm::instruction::exchange(
            &bob.keypair.pubkey(),
            &bob_temp_x,
            &bob.main_y,
            &spl_token::id(),
            &program_id,
        );

        let mut transaction =
            Transaction::new_with_payer(&[exchange_instruction], Some(&bob.keypair.pubkey()));
        transaction.sign(&[&bob.keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        let bob_x_balance = balance_of(bob.main_x, &mut banks_client).await;
        let bob_y_balance = balance_of(bob.main_y, &mut banks_client).await;
        println!("===================================================");
        println!("Bob X balance: {}", bob_x_balance as f64 / ONE_X as f64);
        println!("Bob Y balance: {}", bob_y_balance as f64 / ONE_Y as f64);
        println!("===================================================");
    }

    println!("Final results =====================================");
    let x_vault_balance = balance_of(x_vault_address, &mut banks_client).await;
    let y_vault_balance = balance_of(y_vault_address, &mut banks_client).await;
    println!("===================================================");
    println!("X vault balance: {}", x_vault_balance as f64 / ONE_X as f64);
    println!("Y vault balance: {}", y_vault_balance as f64 / ONE_Y as f64);
    println!("===================================================");

    let alice_y_balance = balance_of(alice.main_y, &mut banks_client).await;
    let bob_y_balance = balance_of(bob.main_y, &mut banks_client).await;
    println!("===================================================");
    println!("Alice Y balance: {}", alice_y_balance as f64 / ONE_Y as f64);
    println!("Bob Y balance: {}", bob_y_balance as f64 / ONE_Y as f64);
    println!("===================================================");
    assert_eq!(alice_y_balance + bob_y_balance, 200 * ONE_Y);

    let (amm_data_account, _) = Pubkey::find_program_address(&[b"data"], &program_id);
    let amm_data = solana_amm::state::AMM::unpack(
        banks_client
            .get_account(amm_data_account)
            .await
            .unwrap()
            .unwrap()
            .data(),
    )
    .unwrap();

    assert_eq!(amm_data.x_amount, 100 * ONE_X);
    assert_eq!(amm_data.y_amount, 10_000 * ONE_Y);
    assert_eq!(amm_data.x_mint, x_mint);
    assert_eq!(amm_data.y_mint, y_mint);
}

async fn mint_token(
    payer: &Keypair,
    banks_client: &mut BanksClient,
    recent_blockhash: Hash,
    decimals: u8,
) -> (Pubkey, Pubkey) {
    let mint_keypair = Keypair::new();

    let rent = banks_client.get_rent().await.unwrap();
    let minimum_balance = rent.minimum_balance(spl_token::state::Mint::LEN);

    let create_mint_account_instruction = system_instruction::create_account(
        &payer.pubkey(),
        &mint_keypair.pubkey(),
        minimum_balance,
        spl_token::state::Mint::LEN as u64,
        &spl_token::ID,
    );

    let init_mint_instruction = spl_token::instruction::initialize_mint(
        &spl_token::ID,
        &mint_keypair.pubkey(),
        &payer.pubkey(),
        None,
        decimals,
    )
    .unwrap();

    let token_account_keypair = Keypair::new();
    let minimum_balance = rent.minimum_balance(spl_token::state::Account::LEN);

    let create_token_account_instruction = system_instruction::create_account(
        &payer.pubkey(),
        &token_account_keypair.pubkey(),
        minimum_balance,
        spl_token::state::Account::LEN as u64,
        &spl_token::ID,
    );

    let init_acccount_instruction = spl_token::instruction::initialize_account(
        &spl_token::ID,
        &token_account_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let mint_instruction = spl_token::instruction::mint_to(
        &spl_token::ID,
        &mint_keypair.pubkey(),
        &token_account_keypair.pubkey(),
        &payer.pubkey(),
        &[&payer.pubkey()],
        10_000_000 * 10u64.pow(decimals as u32),
    )
    .unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[
            create_mint_account_instruction,
            init_mint_instruction,
            create_token_account_instruction,
            init_acccount_instruction,
            mint_instruction,
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(
        &[&payer, &mint_keypair, &token_account_keypair],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    (mint_keypair.pubkey(), token_account_keypair.pubkey())
}

async fn create_token_account(
    payer: &Keypair,
    mint: &Pubkey,
    banks_client: &mut BanksClient,
    recent_blockhash: Hash,
) -> Pubkey {
    let rent = banks_client.get_rent().await.unwrap();

    let token_account_keypair = Keypair::new();
    let minimum_balance = rent.minimum_balance(spl_token::state::Account::LEN);

    let create_token_account_instruction = system_instruction::create_account(
        &payer.pubkey(),
        &token_account_keypair.pubkey(),
        minimum_balance,
        spl_token::state::Account::LEN as u64,
        &spl_token::ID,
    );

    let init_acccount_instruction = spl_token::instruction::initialize_account(
        &spl_token::ID,
        &token_account_keypair.pubkey(),
        &mint,
        &payer.pubkey(),
    )
    .unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[create_token_account_instruction, init_acccount_instruction],
        Some(&payer.pubkey()),
    );

    transaction.sign(&[payer, &token_account_keypair], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    token_account_keypair.pubkey()
}

async fn transfer_tokens(
    payer: &Keypair,
    banks_client: &mut BanksClient,
    recent_blockhash: Hash,
    sender: &Pubkey,
    recipient: &Pubkey,
    amount: u64,
) {
    let transfer_instruction = spl_token::instruction::transfer(
        &spl_token::ID,
        &sender,
        &recipient,
        &payer.pubkey(),
        &[&payer.pubkey()],
        amount as u64,
    )
    .unwrap();

    let mut transaction =
        Transaction::new_with_payer(&[transfer_instruction], Some(&payer.pubkey()));
    transaction.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

async fn balance_of(address: Pubkey, banks_client: &mut BanksClient) -> u64 {
    spl_token::state::Account::unpack(
        banks_client
            .get_account(address)
            .await
            .unwrap()
            .unwrap()
            .data(),
    )
    .unwrap()
    .amount as u64
}
