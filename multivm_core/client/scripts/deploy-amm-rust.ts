import fs from "fs";
import { ethers } from "hardhat";
import { formatEther, parseEther } from "ethers";
import { GetPoolsSchema, view } from "./utils";
import { deserialize } from "borsh";
import { Token, AMM } from "../typechain-types/contracts";
import {
  createUserSafe,
  deployRustAMMContract,
  deployTokenContract,
  rustAmmContractAddLiquidity,
  rustAmmContractAddPool,
  rustAmmContractInit,
  rustAmmContractRemoveLiquidity,
  rustAmmContractSwap,
} from "./helpers";

const AMM_CONTRACT_SRC = "../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/amm";
const bytecode = fs.readFileSync(AMM_CONTRACT_SRC);
const ammPrivateKey = "d7607486f86dd992de52240a4874c4a9a49fdcefab77897044ec7da3498993b5";
const amm = new ethers.Wallet(new ethers.SigningKey("0x" + ammPrivateKey), ethers.provider);
const owner = new ethers.Wallet(new ethers.SigningKey("0x" + "afdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890"), ethers.provider);
const user = new ethers.Wallet(new ethers.SigningKey("0x" + "af1a53abf88f4821840a2934f3facfc8b1827cccd7f2e331375d2faf8c1032d2"), ethers.provider);

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
    await createUserSafe("owner.multivm", owner.address);
    await createUserSafe("user.multivm", user.address);
    await createUserSafe("amm.multivm", amm.address);
    console.table([
      { name: "owner.multivm", address: owner.address },
      { name: "user.multivm", address: user.address },
      { name: "amm.multivm", address: amm.address },
    ]);
  }

  async function deployTokens() {
    console.log("\nDeploying & Approving tokens...");
    token1 = await deployTokenContract({ name: "Token 1", symbol: "TKN1", supply: tokensSupplyAmount, signer: owner });
    token1Address = await token1.getAddress();
    await token1.approve(user.address, tokensSupplyAmount);
    await token1.approve(amm.address, tokensSupplyAmount);

    token2 = await deployTokenContract({ name: "Token 1", symbol: "TKN1", supply: tokensSupplyAmount, signer: owner });
    token2Address = await token2.getAddress();
    await token2.approve(user.address, tokensSupplyAmount);
    await token2.approve(amm.address, tokensSupplyAmount);
    console.log([token1Address, token2Address]);

    await token1.connect(user).approve(amm.address, tokensSupplyAmount);
    await token1.connect(amm).approve(user.address, tokensSupplyAmount);
    await token2.connect(user).approve(amm.address, tokensSupplyAmount);
    await token2.connect(amm).approve(user.address, tokensSupplyAmount);
  }

  async function transferTokens() {
    console.log("\nTransfer token1 from owner to user...");
    await token1.connect(owner).transfer(user.address, transferAmount);
  }

  async function getTokenMetadata() {
    console.log("\n -- [token_balances] loading...");
    let ot1 = await token1.balanceOf(owner.address);
    let ot2 = await token2.balanceOf(owner.address);
    let ut1 = await token1.balanceOf(user.address);
    let ut2 = await token2.balanceOf(user.address);
    let ammt1 = await token1.balanceOf(amm.address);
    let ammt2 = await token2.balanceOf(amm.address);
    console.table([
      { signer: "owner", token1: ot1, token2: ot2, token1_fmt: +formatEther(ot1), token2_fmt: +formatEther(ot2) },
      { signer: "amm", token1: ammt1, token2: ammt2, token1_fmt: +formatEther(ammt1), token2_fmt: +formatEther(ammt2) },
      { signer: "user", token1: ut1, token2: ut2, token1_fmt: +formatEther(ut1), token2_fmt: +formatEther(ut2) },
    ]);
  }

  async function deployAMM() {
    console.log("\nDeploying AMM contract...");
    ammContract = await deployRustAMMContract({ mvmAddress: "amm.multivm", privateKey: ammPrivateKey, byteCode: bytecode, address: amm.address, signer: amm });
    console.log(`AMM deployed`, await ammContract.getAddress());
  }

  async function ammInit() {
    console.log(`\n —— [init] send transaction...`);
    await rustAmmContractInit({ contract: ammContract, signer: owner });
    console.log(` —— [init] ready!`);
  }

  async function ammAddPool(t0Address: string, t1Address: string) {
    console.log(`\n —— [add_pool] send transaction...`);
    await rustAmmContractAddPool({ token0: t0Address, token1: t1Address, contract: ammContract, signer: owner });
    console.log(` —— [add_pool] ready!`);
  }

  async function ammAddLiquidity(poolId: number, am0: number, am1: number) {
    console.log(`\n —— [add_liquidity] send transaction...`);
    await rustAmmContractAddLiquidity({ poolId: poolId, amount0: am0, amount1: am1, contract: ammContract, signer: owner });
    console.log(` —— [add_liquidity] ready!`);
  }

  async function ammRemoveLiquidity(poolId: number) {
    console.log(`\n —— [remove_liquidity] send transaction...`);
    await rustAmmContractRemoveLiquidity({ poolId: poolId, contract: ammContract, signer: owner });
    console.log(` —— [remove_liquidity] ready!`);
  }

  async function ammSwap(poolId: number, am0In: number, am1In: number) {
    console.log(`\n —— [swap] send transaction...`);
    //! signer = user
    await rustAmmContractSwap({ poolId: poolId, amount0_in: am0In, amount1_in: am1In, contract: ammContract, signer: user });
    console.log(` —— [swap] ready!`);
  }

  async function getPoolsMetadata(fullInfo = false) {
    console.log("\n -- [get_pools] loading...");
    const poolsRAW = await view("amm.multivm", "get_pools", []);
    const data = deserialize(GetPoolsSchema, Buffer.from(poolsRAW.result.replace("0x", ""), "hex"));
    if (fullInfo) {
      console.dir(data);
    } else {
      console.dir(data?.map((p) => ({ id: p.id, reserve0: p.reserve0, reserve1: p.reserve1, total_shares: p.total_shares })));
    }
  }

  // ——————— Start
  await createAccounts();
  await deployTokens();
  await transferTokens().then(async () => await getTokenMetadata());
  await deployAMM();
  await ammInit();
  await ammAddPool(token1Address, token2Address).then(async () => await getPoolsMetadata(true));
  await ammAddLiquidity(0, 1_00, 20_000)
    .then(async () => await getTokenMetadata())
    .then(async () => await getPoolsMetadata());

  await ammSwap(0, 1, 0)
    .then(async () => await getTokenMetadata())
    .then(async () => await getPoolsMetadata());
  await ammRemoveLiquidity(0)
    .then(async () => await getTokenMetadata())
    .then(async () => await getPoolsMetadata());
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
