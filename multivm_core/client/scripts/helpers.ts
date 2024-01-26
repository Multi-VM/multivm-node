import { Wallet, formatEther, parseEther } from "ethers";
import {
  SolanaAmmPoolSchema,
  SolanaAmmSchema,
  SolanaAmmStateSchema,
  SolanaContextSchema,
  accountInfo,
  addLiquidityArgs,
  addPoolArgs,
  bigintToBeBytes,
  create_account,
  defaultDeposit,
  defaultGas,
  deploy_contract,
  get_balance,
  initArgs,
  removeLiquidityArgs,
  solana_data,
  swapArgs,
  toHexString,
  transactionSchema,
} from "./utils";
import { ethers, network } from "hardhat";
import { HardhatEthersSigner } from "@nomicfoundation/hardhat-ethers/signers";
import { AMM, WMVM } from "../typechain-types/contracts";
import { deserialize, serialize } from "borsh";
import { PublicKey } from "@solana/web3.js";

export async function getChainMetadata() {
  const chainId = parseInt(await network.provider.send("eth_chainId"));
  console.log("Chain ID:", chainId);
}

export async function createUserSafe(mvm: string, address: string): Promise<boolean> {
  const mvmBalanceString = await get_balance(address);
  const balance = BigInt(mvmBalanceString);
  if (balance > 0) {
    console.info(`${mvm}: ${address} already exist. Balance: ${formatEther(mvmBalanceString)} MVM.`);
    return false;
  }
  await create_account(mvm, address);
  console.log(`${mvm}: ${address} created!`);
  return true;
}

export async function createUsersFromSigners() {
  const signers = await ethers.getSigners();

  for (const [index, signer] of signers.entries()) {
    let signerAadress = await signer.getAddress();
    try {
      await createUserSafe(`user${index}.multivm`, signerAadress);
    } catch (error) {
      console.log(`Error creating user${index}.multivm: ${signerAadress} account!`);
    }
  }
}

export async function deployWrappedContract(signer?: HardhatEthersSigner | Wallet) {
  const wrapped_contract = await ethers.getContractFactory("WMVM");
  const contract = signer ? await wrapped_contract.connect(signer).deploy() : await wrapped_contract.deploy();
  return await contract.waitForDeployment();
}

export async function depositWrapped(contract: WMVM, signer: HardhatEthersSigner | Wallet, amount: string) {
  const signerAddress = signer.getAddress();
  const approveTx = await contract.connect(signer).approve(signerAddress, parseEther(amount));
  await approveTx.wait();
  const depositTx = await contract.connect(signer).deposit({ value: parseEther(amount) });
  await depositTx.wait();
}

export async function deployTokenContract({
  signer,
  name,
  symbol,
  supply = parseEther(String(1_000_000_000)),
}: {
  signer?: HardhatEthersSigner | Wallet;
  name: string;
  symbol: string;
  supply?: bigint;
}) {
  const token_contract = await ethers.getContractFactory("Token");
  const contract = signer ? await token_contract.connect(signer).deploy(name, symbol, supply) : await token_contract.deploy(name, symbol, supply);
  return await contract.waitForDeployment();
}

export async function deployRustAMMContract({
  mvmAddress,
  privateKey,
  byteCode,
  address,
  signer,
}: {
  mvmAddress: string;
  privateKey: string;
  byteCode: Buffer;
  address: string;
  signer: HardhatEthersSigner | Wallet;
}) {
  await deploy_contract(mvmAddress, "mvm", privateKey, toHexString(byteCode));
  return await ethers.getContractAt("AMM", address, signer);
}

// gas: 0x5208,
// data: Buffer.from(data.buffer, data.byteOffset, data.byteLength).toString("hex"),
export async function rustAmmContractInit({ contract, signer }: { contract: AMM; signer: HardhatEthersSigner | Wallet }) {
  const contractAddress = await contract.getAddress();
  const data = serialize(transactionSchema, {
    method: "init",
    args: serialize(initArgs, []),
    gas: defaultGas,
    deposit: defaultDeposit,
  });

  const initTx = await signer.sendTransaction({ to: contractAddress, data: data });
  await initTx.wait();
}

export async function rustAmmContractAddPool({ token0, token1, contract, signer }: { token0: string; token1: string; contract: AMM; signer: HardhatEthersSigner | Wallet }) {
  const contractAddress = await contract.getAddress();
  const data = serialize(transactionSchema, {
    method: "add_pool",
    args: serialize(addPoolArgs, {
      token0: token0,
      token1: token1,
    }),
    gas: defaultGas,
    deposit: defaultDeposit,
  });

  const addPoolTx = await signer.sendTransaction({ to: contractAddress, data: data });
  await addPoolTx.wait();
}

export async function rustAmmContractAddLiquidity({
  poolId,
  amount0,
  amount1,
  contract,
  signer,
}: {
  poolId: number;
  amount0: number;
  amount1: number;
  contract: AMM;
  signer: HardhatEthersSigner | Wallet;
}) {
  const contractAddress = await contract.getAddress();
  const data = serialize(transactionSchema, {
    method: "add_liquidity",
    args: serialize(addLiquidityArgs, {
      pool_id: BigInt(poolId),
      amount0: parseEther(String(amount0)),
      amount1: parseEther(String(amount1)),
    }),
    gas: defaultGas,
    deposit: defaultDeposit,
  });

  const addLiquidityTx = await signer.sendTransaction({ to: contractAddress, data: data });
  await addLiquidityTx.wait();
}

export async function rustAmmContractRemoveLiquidity({ poolId, contract, signer }: { poolId: number; contract: AMM; signer: HardhatEthersSigner | Wallet }) {
  const contractAddress = await contract.getAddress();
  const data = serialize(transactionSchema, {
    method: "remove_liquidity",
    args: serialize(removeLiquidityArgs, {
      pool_id: BigInt(poolId),
    }),
    gas: defaultGas,
    deposit: defaultDeposit,
  });

  const removeLiquidityTx = await signer.sendTransaction({ to: contractAddress, data: data });
  await removeLiquidityTx.wait();
}

export async function rustAmmContractSwap({
  poolId,
  amount0_in,
  amount1_in,
  contract,
  signer,
}: {
  poolId: number;
  amount0_in: number;
  amount1_in: number;
  contract: AMM;
  signer: HardhatEthersSigner | Wallet;
}) {
  const contractAddress = await contract.getAddress();
  const data = serialize(transactionSchema, {
    method: "swap",
    args: serialize(swapArgs, {
      pool_id: BigInt(poolId),
      amount0_in: parseEther(String(amount0_in)),
      amount1_in: parseEther(String(amount1_in)),
    }),
    gas: defaultGas,
    deposit: defaultDeposit,
  });

  const swapTx = await signer.sendTransaction({ to: contractAddress, data: data });
  await swapTx.wait();
}

export async function deploySvmAMMContract({
  mvmAddress,
  privateKey,
  byteCode,
  address,
  signer,
}: {
  mvmAddress: string;
  privateKey: string;
  byteCode: Buffer;
  address: string;
  signer: HardhatEthersSigner | Wallet;
}) {
  await deploy_contract(mvmAddress, "svm", privateKey, toHexString(byteCode));
  return await ethers.getContractAt("AMM", address, signer);
}

export async function svmAmmContractInit({ contract, signer }: { contract: AMM; signer: HardhatEthersSigner | Wallet }) {
  const contractAddress = await contract.getAddress();

  const program_id = new PublicKey((await accountInfo(contractAddress))["result"]["solana_address"]);
  const owner_solana_id = new PublicKey((await accountInfo(signer.address))["result"]["solana_address"]);
  const [state_account_id, _] = PublicKey.findProgramAddressSync([Buffer.from("state")], program_id);

  const data = serialize(transactionSchema, {
    method: "init",
    args: serialize(SolanaContextSchema, {
      accounts: [owner_solana_id.toBytes(), state_account_id.toBytes()],
      instruction_data: new Uint8Array([0]),
    }),
    gas: defaultGas,
    deposit: defaultDeposit,
  });

  const initTx = await signer.sendTransaction({ to: contractAddress, data: data });
  await initTx.wait();
}

export async function svmAmmContractAddPool({ token0, token1, contract, signer }: { token0: string; token1: string; contract: AMM; signer: HardhatEthersSigner | Wallet }) {
  const contractAddress = await contract.getAddress();

  const program_id = new PublicKey((await accountInfo(contractAddress))["result"]["solana_address"]);
  const owner_solana_id = new PublicKey((await accountInfo(signer.address))["result"]["solana_address"]);
  const [state_account_id, _] = PublicKey.findProgramAddressSync([Buffer.from("state")], program_id);
  const state_account_data = (await solana_data(program_id.toBase58(), state_account_id.toBase58()))["result"];
  const state = deserialize(SolanaAmmStateSchema, state_account_data);
  const pool_id = state["next_pool_id"];
  const pool_id_bytes = bigintToBeBytes(pool_id, 16);
  const [pool_state_account_id] = PublicKey.findProgramAddressSync([Buffer.from("pool"), pool_id_bytes], program_id);

  const instruction_data = serialize(SolanaAmmSchema, {
    add_pool: {
      token0: token0,
      token1: token1,
    },
  });

  const data = serialize(transactionSchema, {
    method: "add_pool",
    args: serialize(SolanaContextSchema, {
      accounts: [owner_solana_id.toBytes(), state_account_id.toBytes(), pool_state_account_id.toBytes()],
      instruction_data: instruction_data,
    }),
    gas: defaultGas,
    deposit: defaultDeposit,
  });

  const addPoolTx = await signer.sendTransaction({ to: contractAddress, data: data });
  await addPoolTx.wait();
}

export async function svmAmmContractAddLiquidity({
  poolId,
  amount0,
  amount1,
  contract,
  signer,
}: {
  poolId: number;
  amount0: number;
  amount1: number;
  contract: AMM;
  signer: HardhatEthersSigner | Wallet;
}) {
  const contractAddress = await contract.getAddress();

  const program_id = new PublicKey((await accountInfo(contractAddress))["result"]["solana_address"]);
  const owner_solana_id = new PublicKey((await accountInfo(signer.address))["result"]["solana_address"]);

  const pool_id_bytes = bigintToBeBytes(BigInt(poolId), 16);
  const [pool_state_account_id] = PublicKey.findProgramAddressSync([Buffer.from("pool"), pool_id_bytes], program_id);
  const [user_pool_shares_account_id] = PublicKey.findProgramAddressSync([Buffer.from("user_pool_shares"), owner_solana_id.toBytes(), pool_id_bytes], program_id);

  const instruction_data = serialize(SolanaAmmSchema, {
    add_liquidity: {
      amount0: parseEther(String(amount0)),
      amount1: parseEther(String(amount1)),
    },
  });

  const data = serialize(transactionSchema, {
    method: "add_liquidity",
    args: serialize(SolanaContextSchema, {
      accounts: [owner_solana_id.toBytes(), pool_state_account_id.toBytes(), user_pool_shares_account_id.toBytes()],
      instruction_data: instruction_data,
    }),
    gas: defaultGas,
    deposit: defaultDeposit,
  });

  const addLiquidityTx = await signer.sendTransaction({ to: contractAddress, data: data });
  await addLiquidityTx.wait();
}

export async function svmAmmContractSwap({
  poolId,
  amount0_in,
  amount1_in,
  contract,
  signer,
}: {
  poolId: number;
  amount0_in: number;
  amount1_in: number;
  contract: AMM;
  signer: HardhatEthersSigner | Wallet;
}) {
  const contractAddress = await contract.getAddress();

  const program_id = new PublicKey((await accountInfo(contractAddress))["result"]["solana_address"]);
  const owner_solana_id = new PublicKey((await accountInfo(signer.address))["result"]["solana_address"]);

  const pool_id_bytes = bigintToBeBytes(BigInt(poolId), 16);
  const [pool_state_account_id] = PublicKey.findProgramAddressSync([Buffer.from("pool"), pool_id_bytes], program_id);

  const instruction_data = serialize(SolanaAmmSchema, {
    swap: {
      amount0_in: parseEther(String(amount0_in)),
      amount1_in: parseEther(String(amount1_in)),
    },
  });

  const data = serialize(transactionSchema, {
    method: "swap",
    args: serialize(SolanaContextSchema, {
      accounts: [owner_solana_id.toBytes(), pool_state_account_id.toBytes()],
      instruction_data: instruction_data,
    }),
    gas: defaultGas,
    deposit: defaultDeposit,
  });

  const swapTx = await signer.sendTransaction({ to: contractAddress, data: data });
  await swapTx.wait();
}

export async function svmAmmPoolsMetaData({ poolId, contract }: { poolId: number; contract: AMM }) {
  const contractAddress = await contract.getAddress();

  const program_id = new PublicKey((await accountInfo(contractAddress))["result"]["solana_address"]);

  const pool_id_bytes = bigintToBeBytes(BigInt(poolId), 16);
  const [pool_state_account_id] = PublicKey.findProgramAddressSync([Buffer.from("pool"), pool_id_bytes], program_id);

  const pool_state_data = (await solana_data(program_id.toBase58(), pool_state_account_id.toBase58()))["result"];
  const pool_state = deserialize(SolanaAmmPoolSchema, pool_state_data);

  return pool_state;
}
