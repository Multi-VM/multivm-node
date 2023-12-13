import fs from "fs";
import { ethers, network } from "hardhat";
import { formatEther, parseEther } from "ethers";
import { GetPoolsSchema, addLiquidityArgs, addPoolArgs, create_account, deploy_contract, initArgs, removeLiquidityArgs, swapArgs, toHexString, transactionSchema, view } from "./utils";
import { deserialize, serialize } from "borsh";
import { Token, AMM } from "../typechain-types/contracts";

const AMM_CONTRACT_SRC = "../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/amm";
const bytecode = fs.readFileSync(AMM_CONTRACT_SRC);
const ammPrivateKey = "d7607486f86dd992de52240a4874c4a9a49fdcefab77897044ec7da3498993b5";
const amm = new ethers.Wallet(new ethers.SigningKey("0x" + ammPrivateKey), ethers.provider);
const owner = new ethers.Wallet(new ethers.SigningKey("0x" + "afdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890"), ethers.provider);
const user = new ethers.Wallet(new ethers.SigningKey("0x" + "af1a53abf88f4821840a2934f3facfc8b1827cccd7f2e331375d2faf8c1032d2"), ethers.provider);

async function main() {
  // Variables
  // Transactions
  let defaultGas = BigInt(300_000);
  let defaultDeposit = BigInt(0);

  // AMM
  let ammContract: AMM;

  // Token related
  const tokensSupplyAmount = parseEther(String(1_000_000));
  const transferAmount = parseEther(String(1_000));
  let token1: Token;
  let token2: Token;
  let token1Address: string;
  let token2Address: string;

  async function getChainMetadata() {
    const chainId = parseInt(await network.provider.send("eth_chainId"));
    console.log("Chain ID:", chainId);
  }

  async function createAccounts() {
    // !You need to create user with this alias first
    console.log("\nCreating users...");
    await create_account("owner.multivm", owner.address);
    await create_account("user.multivm", user.address);
    await create_account("amm.multivm", amm.address);
    console.table([
      { name: "owner.multivm", address: owner.address },
      { name: "user.multivm", address: user.address },
      { name: "amm.multivm", address: amm.address },
    ]);
  }

  async function deployTokens() {
    console.log("\nDeploying & Approving tokens...");
    const TOKEN_FACTORY = await ethers.getContractFactory("Token");
    token1 = await TOKEN_FACTORY.connect(owner).deploy("Token1", "TKN1", tokensSupplyAmount);
    token1Address = await token1.getAddress();
    await token1.approve(user.address, tokensSupplyAmount);
    await token1.approve(amm.address, tokensSupplyAmount);

    token2 = await TOKEN_FACTORY.connect(owner).deploy("Token2", "TKN2", tokensSupplyAmount);
    await token2.approve(user.address, tokensSupplyAmount);
    await token2.approve(amm.address, tokensSupplyAmount);
    token2Address = await token2.getAddress();
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
    await deploy_contract("amm.multivm", ammPrivateKey, toHexString(bytecode));
    ammContract = await ethers.getContractAt("AMM", amm.address, amm);
    console.log(`AMM deployed`, await ammContract.getAddress());
  }

  async function ammInit() {
    console.log(`\n —— [init] send transaction...`);

    // gas: 0x5208,
    // data: Buffer.from(data.buffer, data.byteOffset, data.byteLength).toString("hex"),

    const data = serialize(transactionSchema, {
      method: "init",
      args: serialize(initArgs, []),
      gas: defaultGas,
      deposit: defaultDeposit,
    });

    const initTx = await owner.sendTransaction({ to: ammContract, data: data });
    // console.dir(initTx);
    console.log(` —— [init] ready!`);
  }

  async function ammAddPool() {
    console.log(`\n —— [add_pool] send transaction...`);
    const data = serialize(transactionSchema, {
      method: "add_pool",
      args: serialize(addPoolArgs, {
        token0: token1Address,
        token1: token2Address,
      }),
      gas: defaultGas,
      deposit: defaultDeposit,
    });

    const addPoolTx = await owner.sendTransaction({
      to: ammContract,
      data: data,
    });

    // console.dir(addPoolTx);
    console.log(` —— [add_pool] ready!`);
  }

  async function ammAddLiquidity() {
    console.log(`\n —— [add_liquidity] send transaction...`);

    const data = serialize(transactionSchema, {
      method: "add_liquidity",
      args: serialize(addLiquidityArgs, {
        pool_id: BigInt(0),
        amount0: parseEther(String(1_00)),
        amount1: parseEther(String(20_000)),
      }),
      gas: defaultGas,
      deposit: defaultDeposit,
    });

    const addLiquidityTx = await owner.sendTransaction({
      to: ammContract,
      data: data,
    });

    // console.dir(addLiquidityTx);
    console.log(` —— [add_liquidity] ready!`);
  }

  async function ammRemoveLiquidity() {
    console.log(`\n —— [remove_liquidity] send transaction...`);

    const data = serialize(transactionSchema, {
      method: "remove_liquidity",
      args: serialize(removeLiquidityArgs, {
        pool_id: BigInt(0),
      }),
      gas: defaultGas,
      deposit: defaultDeposit,
    });

    const removeLiquidityTx = await owner.sendTransaction({
      to: ammContract,
      data: data,
    });

    // console.dir(addLiquidityTx);
    console.log(` —— [remove_liquidity] ready!`);
  }

  async function ammSwap() {
    console.log(`\n —— [swap] send transaction...`);

    const data = serialize(transactionSchema, {
      method: "swap",
      args: serialize(swapArgs, {
        pool_id: BigInt(0),
        amount0_in: parseEther(String(1)),
        amount1_in: BigInt(0),
      }),
      gas: defaultGas,
      deposit: defaultDeposit,
    });

    const swapTx = await user.sendTransaction({
      to: ammContract,
      data: data,
    });

    // console.dir(swapTx);
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
  // await getChainMetadata();
  await createAccounts();
  await deployTokens();
  await transferTokens().then(async () => await getTokenMetadata());
  await deployAMM();
  await ammInit();
  await ammAddPool().then(async () => await getPoolsMetadata(true));
  await ammAddLiquidity()
    .then(async () => await getTokenMetadata())
    .then(async () => await getPoolsMetadata());

  return;

  await ammSwap()
    .then(async () => await getTokenMetadata())
    .then(async () => await getPoolsMetadata());
  await ammRemoveLiquidity()
    .then(async () => await getTokenMetadata())
    .then(async () => await getPoolsMetadata());
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
