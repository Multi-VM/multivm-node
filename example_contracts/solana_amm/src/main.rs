#![no_main]

use std::str::FromStr;

use ethabi::ethereum_types::U256;
use instruction::{AddLiquidityRequest, AddPoolRequest, SwapRequest};
use num::integer::Roots;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh::{self, BorshDeserialize, BorshSerialize},
    entrypoint,
    entrypoint::ProgramResult,
    msg, multivm_sdk,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::instruction::Instruction;

mod instruction;

const ABI_BYTES: &[u8] = include_bytes!("../../../multivm_core/etc/evm_contracts/erc20.abi");

#[derive(BorshSerialize, BorshDeserialize, Default)]
pub struct State {
    pub next_pool_id: u128,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct Pool {
    pub id: u128,
    pub token0: Token,
    pub token1: Token,
    pub reserve0: u128,
    pub reserve1: u128,
    pub total_shares: u128,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct Token {
    pub symbol: String,
    pub address: String,
    pub decimals: u8,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct MultiVmAccount {
    internal_id: u128,
    pub evm_address: multivm_sdk::multivm_primitives::EvmAddress,
    pub multivm_account_id: Option<multivm_sdk::multivm_primitives::MultiVmAccountId>,
    pub solana_address: Option<multivm_sdk::multivm_primitives::SolanaAddress>,
    pub executable: Option<Executable>,
    pub balance: u128,
    pub nonce: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub enum Executable {
    Evm(),
    MultiVm(MultiVmExecutable),
    Solana(SolanaExecutable),
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct MultiVmExecutable {
    pub image_id: [u32; 8],
}
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct SolanaExecutable {
    pub image_id: [u32; 8],
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Solana AMM entrypoint");

    let instruction: Instruction =
        borsh::from_slice(&instruction_data).map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        Instruction::Init() => init(program_id, accounts),
        Instruction::AddPool(request) => add_pool(program_id, accounts, request),
        Instruction::AddLiquidity(request) => add_liquidity(program_id, accounts, request),
        Instruction::RemoveLiquidity() => remove_liquidity(program_id, accounts),
        Instruction::Swap(request) => swap(program_id, accounts, request),
    }
}

pub fn init(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let _signer_account = next_account_info(accounts_iter)?;
    let state_account = next_account_info(accounts_iter)?;
    {
        if state_account.owner != program_id {
            msg!("State account does not have the correct program id");
            return Err(ProgramError::IncorrectProgramId);
        }
        let expected_state_account_key = Pubkey::find_program_address(&[b"state"], program_id).0;
        if state_account.key != &expected_state_account_key {
            msg!("State account does not have the correct address");
            return Err(ProgramError::IncorrectProgramId);
        }
    }
    let state = State { next_pool_id: 1 };
    borsh::to_writer(&mut &mut state_account.try_borrow_mut_data()?[..], &state)?;

    Ok(())
}

pub fn add_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    request: AddPoolRequest,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let _signer_account = next_account_info(accounts_iter)?;
    let state_account = next_account_info(accounts_iter)?;
    {
        if state_account.owner != program_id {
            msg!("State account does not have the correct program id");
            return Err(ProgramError::IncorrectProgramId);
        }
        let expected_state_account_key = Pubkey::find_program_address(&[b"state"], program_id).0;
        if state_account.key != &expected_state_account_key {
            msg!("State account does not have the correct address");
            return Err(ProgramError::IncorrectProgramId);
        }
    }
    let mut state = State::deserialize(&mut &state_account.try_borrow_data()?[..])?;
    let pool_id = state.next_pool_id;
    state.next_pool_id += 1;

    let pool_account = next_account_info(accounts_iter)?;
    {
        if pool_account.owner != program_id {
            msg!("Pool account does not have the correct program id");
            return Err(ProgramError::IncorrectProgramId);
        }
        let expected_pool_account_key =
            Pubkey::find_program_address(&[b"pool", &pool_id.to_be_bytes()], program_id).0;
        msg!("{:?}", pool_id.to_be_bytes());
        if pool_account.key != &expected_pool_account_key {
            msg!(
                "Pool account does not have the correct address. Expected: {:?}, actual: {:?}",
                expected_pool_account_key,
                pool_account.key
            );
            return Err(ProgramError::IncorrectProgramId);
        }
    }

    let token0 = multivm_sdk::multivm_primitives::EvmAddress::from_str(&request.token0).unwrap();
    let token1 = multivm_sdk::multivm_primitives::EvmAddress::from_str(&request.token1).unwrap();

    let abi = ethabi::Contract::load(ABI_BYTES).unwrap();
    let symbols_function = abi.function("symbol").unwrap();
    let symbols_encoded_input = symbols_function.encode_input(&vec![]).unwrap();
    let decimals_function = abi.function("decimals").unwrap();
    let decimals_encoded_input = decimals_function.encode_input(&vec![]).unwrap();

    let tokens: Vec<Token> = [token0, token1]
        .iter()
        .map(|address| {
            let commitment0 = multivm_sdk::env::cross_contract_call_raw(
                address.clone().into(),
                "symbol".to_string(),
                0,
                symbols_encoded_input.clone(),
            );
            let response_bytes0: Vec<u8> =
                borsh::from_slice(&commitment0.response.unwrap()).unwrap();

            let symbol = symbols_function
                .decode_output(response_bytes0.as_slice())
                .unwrap()
                .first()
                .unwrap()
                .to_string();

            let commitment1 = multivm_sdk::env::cross_contract_call_raw(
                address.clone().into(),
                "decimals".to_string(),
                0,
                decimals_encoded_input.clone(),
            );
            let response_bytes1: Vec<u8> =
                borsh::from_slice(&commitment1.response.unwrap()).unwrap();

            let decimals = decimals_function
                .decode_output(response_bytes1.as_slice())
                .unwrap()
                .first()
                .unwrap()
                .clone()
                .into_uint()
                .unwrap()
                .try_into()
                .unwrap();

            Token {
                symbol,
                address: format!("0x{}", address.to_string()),
                decimals,
            }
        })
        .collect();

    let pool = Pool {
        id: pool_id,
        token0: tokens[0].clone(),
        token1: tokens[1].clone(),
        reserve0: 0,
        reserve1: 0,
        total_shares: 0,
    };

    borsh::to_writer(&mut &mut pool_account.try_borrow_mut_data()?[..], &pool)?;
    borsh::to_writer(&mut &mut state_account.try_borrow_mut_data()?[..], &state)?;

    msg!("Pool created: {:?}", pool);

    Ok(())
}

pub fn add_liquidity(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    request: AddLiquidityRequest,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let user_account = next_account_info(accounts_iter)?;
    // if !user_account.is_signer {
    //     msg!("User account is not signer");
    //     return Err(ProgramError::MissingRequiredSignature);
    // }

    let pool_account = next_account_info(accounts_iter)?;
    let mut pool = {
        if pool_account.owner != program_id {
            msg!("Pool account does not have the correct program id");
            return Err(ProgramError::IncorrectProgramId);
        }
        let pool = Pool::deserialize(&mut &pool_account.try_borrow_data()?[..])?;
        let expected_pool_account_key =
            Pubkey::find_program_address(&[b"pool", &pool.id.to_be_bytes()], program_id).0;
        if pool_account.key != &expected_pool_account_key {
            msg!("Pool account does not have the correct address");
            return Err(ProgramError::IncorrectProgramId);
        }
        pool
    };

    let user_pool_shares_account = next_account_info(accounts_iter)?;
    let mut user_pool_shares: u128 = {
        if user_pool_shares_account.owner != program_id {
            msg!("User pool shares account does not have the correct program id");
            return Err(ProgramError::IncorrectProgramId);
        }
        let expected_user_shares_account_key = Pubkey::find_program_address(
            &[
                b"user_pool_shares",
                &user_account.key.to_bytes(),
                &pool.id.to_be_bytes(),
            ],
            program_id,
        );
        if user_pool_shares_account.key != &expected_user_shares_account_key.0 {
            msg!("User pool shares account does not have the correct address");
            return Err(ProgramError::IncorrectProgramId);
        }
        u128::deserialize(&mut &user_pool_shares_account.try_borrow_data()?[..])?
    };

    let user_multivm_account = {
        let commitment = multivm_sdk::env::cross_contract_call(
            multivm_sdk::multivm_primitives::AccountId::system_meta_contract(),
            "account_info".to_string(),
            0,
            &multivm_sdk::multivm_primitives::AccountId::from(
                multivm_sdk::multivm_primitives::SolanaAddress::from(user_account.key.to_bytes()),
            ),
        );
        borsh::from_slice::<Option<MultiVmAccount>>(&commitment.response.unwrap())
            .unwrap()
            .unwrap()
    };

    let program_multivm_account = {
        let commitment = multivm_sdk::env::cross_contract_call(
            multivm_sdk::multivm_primitives::AccountId::system_meta_contract(),
            "account_info".to_string(),
            0,
            &multivm_sdk::multivm_primitives::AccountId::from(
                multivm_sdk::multivm_primitives::SolanaAddress::from(program_id.to_bytes()),
            ),
        );
        borsh::from_slice::<Option<MultiVmAccount>>(&commitment.response.unwrap())
            .unwrap()
            .unwrap()
    };

    let (amount0, amount1) = if pool.total_shares == 0 {
        (request.amount0, request.amount1)
    } else {
        assert!(
            (request.amount0 != 0) ^ (request.amount1 != 0),
            "You need to specify the amount for only one token {} {}",
            request.amount0,
            request.amount1
        );

        if request.amount0 > 0 {
            (
                request.amount0,
                (U256::from(request.amount0) * U256::from(pool.reserve1)
                    / U256::from(pool.reserve0))
                .as_u128(),
            )
        } else {
            (
                (U256::from(request.amount1) * U256::from(pool.reserve0)
                    / U256::from(pool.reserve1))
                .as_u128(),
                request.amount1,
            )
        }
    };

    let abi = ethabi::Contract::load(ABI_BYTES).unwrap();
    let function = abi.function("transferFrom").unwrap();

    {
        let encoded_input0 = function
            .encode_input(&vec![
                ethabi::Token::Address(user_multivm_account.evm_address.clone().into()),
                ethabi::Token::Address(program_multivm_account.evm_address.clone().into()),
                ethabi::Token::Uint(amount0.into()),
            ])
            .unwrap();
        let token0_address =
            multivm_sdk::multivm_primitives::EvmAddress::from_str(&pool.token0.clone().address)
                .unwrap();
        let commitment = multivm_sdk::env::cross_contract_call_raw(
            token0_address.clone().into(),
            "transferFrom".to_string(),
            0,
            encoded_input0.clone(),
        );
        if commitment.response.is_err() {
            panic!("Can not transfer token {}", token0_address);
        }
    }

    {
        let encoded_input1 = function
            .encode_input(&vec![
                ethabi::Token::Address(user_multivm_account.evm_address.into()),
                ethabi::Token::Address(program_multivm_account.evm_address.into()),
                ethabi::Token::Uint(amount1.into()),
            ])
            .unwrap();
        let token1_address =
            multivm_sdk::multivm_primitives::EvmAddress::from_str(&pool.token1.clone().address)
                .unwrap();
        let commitment = multivm_sdk::env::cross_contract_call_raw(
            token1_address.clone().into(),
            "transferFrom".to_string(),
            0,
            encoded_input1.clone(),
        );
        if commitment.response.is_err() {
            panic!("Can not transfer token {}", token1_address);
        }
    }

    let shares = if pool.total_shares == 0 {
        (amount0 * amount1).sqrt()
    } else {
        (U256::from(amount0) * U256::from(pool.total_shares) / U256::from(pool.reserve0)).as_u128()
    };

    pool.total_shares += shares;
    pool.reserve0 += amount0;
    pool.reserve1 += amount1;

    user_pool_shares += shares;

    borsh::to_writer(
        &mut &mut user_pool_shares_account.try_borrow_mut_data()?[..],
        &user_pool_shares,
    )?;

    borsh::to_writer(&mut &mut pool_account.try_borrow_mut_data()?[..], &pool)?;

    Ok(())
}

pub fn remove_liquidity(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let user_account = next_account_info(accounts_iter)?;
    // if !user_account.is_signer {
    //     msg!("User account is not signer");
    //     return Err(ProgramError::MissingRequiredSignature);
    // }

    let pool_account = next_account_info(accounts_iter)?;
    let mut pool = {
        if pool_account.owner != program_id {
            msg!("Pool account does not have the correct program id");
            return Err(ProgramError::IncorrectProgramId);
        }
        let pool = Pool::deserialize(&mut &pool_account.try_borrow_data()?[..])?;
        let expected_pool_account_key =
            Pubkey::find_program_address(&[b"pool", &pool.id.to_be_bytes()], program_id).0;
        if pool_account.key != &expected_pool_account_key {
            msg!("Pool account does not have the correct address");
            return Err(ProgramError::IncorrectProgramId);
        }
        pool
    };

    let user_pool_shares_account = next_account_info(accounts_iter)?;
    let mut user_pool_shares: u128 = {
        if user_pool_shares_account.owner != program_id {
            msg!("User pool shares account does not have the correct program id");
            return Err(ProgramError::IncorrectProgramId);
        }
        let expected_user_shares_account_key = Pubkey::find_program_address(
            &[
                b"user_pool_shares",
                &user_account.key.to_bytes(),
                &pool.id.to_be_bytes(),
            ],
            program_id,
        );
        if user_pool_shares_account.key != &expected_user_shares_account_key.0 {
            msg!("User pool shares account does not have the correct address");
            return Err(ProgramError::IncorrectProgramId);
        }
        u128::deserialize(&mut &user_pool_shares_account.try_borrow_data()?[..])?
    };

    let user_multivm_account = {
        let commitment = multivm_sdk::env::cross_contract_call(
            multivm_sdk::multivm_primitives::AccountId::system_meta_contract(),
            "account_info".to_string(),
            0,
            &multivm_sdk::multivm_primitives::AccountId::from(
                multivm_sdk::multivm_primitives::SolanaAddress::from(user_account.key.to_bytes()),
            ),
        );
        borsh::from_slice::<Option<MultiVmAccount>>(&commitment.response.unwrap())
            .unwrap()
            .unwrap()
    };

    let amount0 = (U256::from(user_pool_shares) * U256::from(pool.reserve0)
        / U256::from(pool.total_shares))
    .as_u128();
    let amount1 = (U256::from(user_pool_shares) * U256::from(pool.reserve1)
        / U256::from(pool.total_shares))
    .as_u128();

    let abi = ethabi::Contract::load(ABI_BYTES).unwrap();
    let function = abi.function("transfer").unwrap();

    {
        let encoded_input0 = function
            .encode_input(&vec![
                ethabi::Token::Address(user_multivm_account.evm_address.clone().into()),
                ethabi::Token::Uint(amount0.into()),
            ])
            .unwrap();
        let token0_address =
            multivm_sdk::multivm_primitives::EvmAddress::from_str(&pool.token0.clone().address)
                .unwrap();
        let commitment = multivm_sdk::env::cross_contract_call_raw(
            token0_address.clone().into(),
            "transfer".to_string(),
            0,
            encoded_input0.clone(),
        );
        if commitment.response.is_err() {
            panic!("Can not send token {}", token0_address);
        }
    }

    {
        let encoded_input1 = function
            .encode_input(&vec![
                ethabi::Token::Address(user_multivm_account.evm_address.clone().into()),
                ethabi::Token::Uint(amount1.into()),
            ])
            .unwrap();
        let token1_address =
            multivm_sdk::multivm_primitives::EvmAddress::from_str(&pool.token1.clone().address)
                .unwrap();
        let commitment = multivm_sdk::env::cross_contract_call_raw(
            token1_address.clone().into(),
            "transfer".to_string(),
            0,
            encoded_input1.clone(),
        );
        if commitment.response.is_err() {
            panic!("Can not send token {}", token1_address);
        }
    }

    pool.total_shares -= user_pool_shares;
    pool.reserve0 -= amount0;
    pool.reserve1 -= amount1;

    user_pool_shares = 0;

    borsh::to_writer(
        &mut &mut user_pool_shares_account.try_borrow_mut_data()?[..],
        &user_pool_shares,
    )?;

    borsh::to_writer(&mut &mut pool_account.try_borrow_mut_data()?[..], &pool)?;

    Ok(())
}

pub fn swap(program_id: &Pubkey, accounts: &[AccountInfo], request: SwapRequest) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let user_account = next_account_info(accounts_iter)?;
    // if !user_account.is_signer {
    //     msg!("User account is not signer");
    //     return Err(ProgramError::MissingRequiredSignature);
    // }

    let pool_account = next_account_info(accounts_iter)?;
    let mut pool = {
        if pool_account.owner != program_id {
            msg!("Pool account does not have the correct program id");
            return Err(ProgramError::IncorrectProgramId);
        }
        let pool = Pool::deserialize(&mut &pool_account.try_borrow_data()?[..])?;
        let expected_pool_account_key =
            Pubkey::find_program_address(&[b"pool", &pool.id.to_be_bytes()], program_id).0;
        if pool_account.key != &expected_pool_account_key {
            msg!("Pool account does not have the correct address");
            return Err(ProgramError::IncorrectProgramId);
        }
        pool
    };

    let user_multivm_account = {
        let commitment = multivm_sdk::env::cross_contract_call(
            multivm_sdk::multivm_primitives::AccountId::system_meta_contract(),
            "account_info".to_string(),
            0,
            &multivm_sdk::multivm_primitives::AccountId::from(
                multivm_sdk::multivm_primitives::SolanaAddress::from(user_account.key.to_bytes()),
            ),
        );
        borsh::from_slice::<Option<MultiVmAccount>>(&commitment.response.unwrap())
            .unwrap()
            .unwrap()
    };

    let program_multivm_account = {
        let commitment = multivm_sdk::env::cross_contract_call(
            multivm_sdk::multivm_primitives::AccountId::system_meta_contract(),
            "account_info".to_string(),
            0,
            &multivm_sdk::multivm_primitives::AccountId::from(
                multivm_sdk::multivm_primitives::SolanaAddress::from(program_id.to_bytes()),
            ),
        );
        borsh::from_slice::<Option<MultiVmAccount>>(&commitment.response.unwrap())
            .unwrap()
            .unwrap()
    };
    let token0_address =
        multivm_sdk::multivm_primitives::EvmAddress::from_str(&pool.token0.clone().address)
            .unwrap();
    let token1_address =
        multivm_sdk::multivm_primitives::EvmAddress::from_str(&pool.token1.clone().address)
            .unwrap();

    let (reserve_in, reserve_out, amount_in, token_in, token_out) = if request.amount0_in > 0 {
        (
            pool.reserve0,
            pool.reserve1,
            request.amount0_in,
            token0_address,
            token1_address,
        )
    } else {
        (
            pool.reserve1,
            pool.reserve0,
            request.amount1_in,
            token1_address,
            token0_address,
        )
    };

    let amount_out = (U256::from(reserve_out) * U256::from(amount_in)
        / (U256::from(reserve_in + amount_in)))
    .as_u128();

    let abi = ethabi::Contract::load(ABI_BYTES).unwrap();

    {
        let transfer_from_function = abi.function("transferFrom").unwrap();
        let encoded_input0 = transfer_from_function
            .encode_input(&vec![
                ethabi::Token::Address(user_multivm_account.evm_address.clone().into()),
                ethabi::Token::Address(program_multivm_account.evm_address.clone().into()),
                ethabi::Token::Uint(amount_in.into()),
            ])
            .unwrap();
        let commitment = multivm_sdk::env::cross_contract_call_raw(
            token_in.clone().into(),
            "transferFrom".to_string(),
            0,
            encoded_input0.clone(),
        );
        if commitment.response.is_err() {
            panic!("Can not transfer token {}", token_in);
        }
    }

    {
        let transfer_function = abi.function("transfer").unwrap();
        let encoded_input1 = transfer_function
            .encode_input(&vec![
                ethabi::Token::Address(user_multivm_account.evm_address.into()),
                ethabi::Token::Uint(amount_out.into()),
            ])
            .unwrap();
        let commitment = multivm_sdk::env::cross_contract_call_raw(
            token_out.clone().into(),
            "transfer".to_string(),
            0,
            encoded_input1.clone(),
        );
        if commitment.response.is_err() {
            panic!("Can not send token {}", token_out);
        }
    }

    if request.amount0_in > 0 {
        pool.reserve0 += amount_in;
        pool.reserve1 -= amount_out;
    } else {
        pool.reserve0 -= amount_out;
        pool.reserve1 += amount_in;
    }

    borsh::to_writer(&mut &mut pool_account.try_borrow_mut_data()?[..], &pool)?;

    Ok(())
}
