import fs from "fs";
import { ethers, network } from "hardhat";
import { formatEther, parseEther } from "ethers";
import { addLiquidityArgs, addPoolArgs, create_account, deploy_contract, initArgs, swapArgs, toHexString, transactionSchema, view } from "./utils";
import { deserialize, serialize } from "borsh";
import { Token, AMM } from "../typechain-types/contracts";

const AMM_CONTRACT_SRC = "../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/amm";
const bytecode = fs.readFileSync(AMM_CONTRACT_SRC);
// 2 account
const privateKey = "afdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890";
const signer = new ethers.SigningKey("0x" + privateKey);
const owner = new ethers.Wallet(signer, ethers.provider);
const ownerMVAddress = "user1.multivm";

const privateKey1 = "af1a53abf88f4821840a2934f3facfc8b1827cccd7f2e331375d2faf8c1032d2";
const signer1 = new ethers.SigningKey("0x" + privateKey1);
const user = new ethers.Wallet(signer1, ethers.provider);
const userMVAddress = "user2.multivm";

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
    // !You need ammAddLiquidity();
    await create_account(ownerMVAddress, owner.address);
    await create_account(userMVAddress, user.address);

    console.log("Users created!");
    console.log(ownerMVAddress, owner.address);
    console.log(userMVAddress, user.address);
  }

  async function deployTokens() {
    console.log("\nDeploying tokens...");
    const TOKEN_FACTORY = await ethers.getContractFactory("Token");
    token1 = await TOKEN_FACTORY.connect(owner).deploy("Token1", "TKN1", tokensSupplyAmount);
    token1Address = await token1.getAddress();

    token2 = await TOKEN_FACTORY.connect(owner).deploy("Token2", "TKN2", tokensSupplyAmount);
    token2Address = await token2.getAddress();

    console.log("Tokens deployed:");
    console.log(token1Address);
    console.log(token2Address);

    await token1.connect(owner).approve(user.address, tokensSupplyAmount);
    console.log("Token 1 Approved...");

    await token2.connect(owner).approve(user.address, tokensSupplyAmount);
    console.log("Token 2 Approved...");
  }

  async function transferTokens() {
    console.log("\nTransfer token1 from owner to user...");
    await token1.connect(owner).transfer(user.address, transferAmount);
    console.log("Transfer complete");
  }

  async function tokenMetadata() {
    console.log("\n -- [token_balances] loading...");
    let ot1 = await token1.balanceOf(owner.address);
    let ot2 = await token2.balanceOf(owner.address);
    let ut1 = await token1.balanceOf(user.address);
    let ut2 = await token2.balanceOf(user.address);
    console.table([
      { signer: "owner", token1: ot1, token2: ot2, token1_fmt: +formatEther(ot1), token2_fmt: +formatEther(ot2) },
      { signer: "user", token1: ut1, token2: ut2, token1_fmt: +formatEther(ut1), token2_fmt: +formatEther(ut2) },
    ]);
  }

  async function deployAMM() {
    console.log("\nDeploying AMM contract...");
    await deploy_contract("user2.multivm", privateKey1, toHexString(bytecode));
    ammContract = await ethers.getContractAt("AMM", user.address, user);
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

    const initTx = await user.sendTransaction({ to: ammContract, data: data });
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

    const addPoolTx = await user.sendTransaction({
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
        amount0: BigInt(1_000),
        amount1: BigInt(0),
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

  async function ammRemoveLiquidity() {}

  async function ammSwap() {
    console.log(`\n —— [swap] send transaction...`);

    const data = serialize(transactionSchema, {
      method: "swap",
      args: serialize(swapArgs, {
        pool_id: BigInt(0),
        amount0_in: BigInt(1_000),
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

  // ——————— Start
  // await getChainMetadata();
  await createAccounts();
  await deployTokens();
  await transferTokens().then(async () => await tokenMetadata());
  await deployAMM();
  await ammInit();
  await ammAddPool();
  await ammAddLiquidity().then(async () => await tokenMetadata());
  // await ammSwap().then(async () => await tokenMetadata());

  return;

  // try {
  //   const pools = await view(owner.address, "get_pools", []);
  //   const result: string = pools.result;
  //   console.log(result);
  //   const res = deserialize({
  //     array: {
  //       type: {
  //         struct: {
  //           id: "u128",
  //           token0: { struct: { symbol: "string", address: "string" } },
  //           token1: { struct: { symbol: "string", address: "string" } },
  //           reserve0: "u128",
  //           reserve1: "u128",
  //           total_shares: "u128",
  //         },
  //       }
  //     }
  //   },
  //   Buffer.from(result.replace("0x", ""), "hex")
  // );
  //   console.log(res);
  // } catch (error) {
  //   console.error(error);
  // }

  // read pool
  // try {
  //   const pools = await view("user1.multivm", "get_pools", []);
  //   console.log(pools);
  // } catch (error) {
  //   console.error(error);
  // }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
