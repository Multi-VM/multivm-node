import { Wallet, formatEther, parseEther } from "ethers";
import {
  addLiquidityArgs,
  addPoolArgs,
  create_account,
  defaultDeposit,
  defaultGas,
  deploy_contract,
  get_balance,
  initArgs,
  removeLiquidityArgs,
  swapArgs,
  toHexString,
  transactionSchema,
} from "./utils";
import { ethers, network } from "hardhat";
import { HardhatEthersSigner } from "@nomicfoundation/hardhat-ethers/signers";
import { AMM, WMVM } from "../typechain-types/contracts";
import { serialize } from "borsh";

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

export async function deployAMMContract({
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
  await deploy_contract(mvmAddress, privateKey, toHexString(byteCode));
  return await ethers.getContractAt("AMM", address, signer);
}

// gas: 0x5208,
// data: Buffer.from(data.buffer, data.byteOffset, data.byteLength).toString("hex"),
export async function ammContractInit({ contract, signer }: { contract: AMM; signer: HardhatEthersSigner | Wallet }) {
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

export async function ammContractAddPool({ token0, token1, contract, signer }: { token0: string; token1: string; contract: AMM; signer: HardhatEthersSigner | Wallet }) {
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

export async function ammContractAddLiquidity({
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

export async function ammContractRemoveLiquidity({ poolId, contract, signer }: { poolId: number; contract: AMM; signer: HardhatEthersSigner | Wallet }) {
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

export async function ammContractSwap({
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
