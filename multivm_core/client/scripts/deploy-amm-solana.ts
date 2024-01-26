import fs from "fs";
import { ethers } from "hardhat";
import { parseEther } from "ethers";
import { Token, AMM } from "../typechain-types/contracts";
import { createUserSafe, deploySvmAMMContract, deployTokenContract, svmAmmContractAddLiquidity, svmAmmContractAddPool, svmAmmContractInit, svmAmmContractSwap, svmAmmPoolsMetaData } from "./helpers";

const AMM_CONTRACT_SRC = "../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/solana_amm";
const bytecode = fs.readFileSync(AMM_CONTRACT_SRC);
// 2 account
const privateKey = "afdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890";
const signer = new ethers.SigningKey("0x" + privateKey);
const owner = new ethers.Wallet(signer, ethers.provider);

// AMM
let ammContract: AMM;

// Token related
const tokensSupplyAmount = parseEther(String(1_000_000));
const transferAmount = parseEther(String(1_000));
let token1: Token;
let token2: Token;
let token1Address: string;
let token2Address: string;

async function main() {
  async function createAccounts() {
    // !You need to create user with this alias first
    console.log("\nCreating users...");
    await createUserSafe("solana_amm.multivm", owner.address);
    console.table([{ name: "solana_amm.multivm", address: owner.address }]);
  }

  async function deployTokens() {
    console.log("\nDeploying & Approving tokens...");
    token1 = await deployTokenContract({ name: "Token 1", symbol: "TKN1", supply: tokensSupplyAmount, signer: owner });
    token1Address = await token1.getAddress();
    await token1.approve(owner.address, tokensSupplyAmount);

    token2 = await deployTokenContract({ name: "Token 1", symbol: "TKN1", supply: tokensSupplyAmount, signer: owner });
    token2Address = await token2.getAddress();
    await token2.approve(owner.address, tokensSupplyAmount);
    console.log([token1Address, token2Address]);
  }

  async function transferTokens() {
    console.log("\nTransfer token1 from owner to user...");
    await token1.connect(owner).transfer("0x121348F398681B4d021826EB6423805df7CD25D9", transferAmount);
  }

  async function deployAMM() {
    console.log("\nDeploying AMM contract...");
    ammContract = await deploySvmAMMContract({ mvmAddress: "solana_amm.multivm", privateKey: privateKey, byteCode: bytecode, address: owner.address, signer: owner });
    console.log(`AMM deployed`, await ammContract.getAddress());
  }

  async function ammInit() {
    console.log(`\n —— [init] send transaction...`);
    await svmAmmContractInit({
      contract: ammContract,
      signer: owner,
    });
    console.log(` —— [init] ready!`);
  }

  async function ammAddPool(t0Address: string, t1Address: string) {
    console.log(`\n —— [add_pool] send transaction...`);
    await svmAmmContractAddPool({ token0: t0Address, token1: t1Address, contract: ammContract, signer: owner });
    console.log(` —— [add_pool] ready!`);
  }

  async function ammAddLiquidity(poolId: number, am0: number, am1: number) {
    console.log(`\n —— [add_liquidity] send transaction...`);
    await svmAmmContractAddLiquidity({ poolId: poolId, amount0: am0, amount1: am1, contract: ammContract, signer: owner });
    console.log(` —— [add_liquidity] ready!`);
  }

  async function ammSwap(poolId: number, am0In: number, am1In: number) {
    console.log(`\n —— [swap] send transaction...`);
    //! signer = user
    await svmAmmContractSwap({ poolId: poolId, amount0_in: am0In, amount1_in: am1In, contract: ammContract, signer: owner });
    console.log(` —— [swap] ready!`);
  }

  async function ammPoolState(poolId: number) {
    const poolState = await svmAmmPoolsMetaData({ poolId: poolId, contract: ammContract });
    console.info(poolState);
  }

  await createAccounts();
  await deployTokens();
  await transferTokens();
  await deployAMM();
  await ammInit();
  await ammAddPool(token1Address, token2Address);
  await ammAddLiquidity(1, 100, 20_000);
  await ammSwap(1, 1, 0);
  await ammPoolState(1);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
