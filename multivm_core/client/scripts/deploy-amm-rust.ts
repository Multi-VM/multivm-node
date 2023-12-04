import fs from "fs";
import { ethers, network } from "hardhat";
import { parseEther } from "ethers";
import {
  addPoolArgs,
  create_account,
  deploy_contract,
  initArgs,
  toHexString,
  transactionSchema,
  view,
} from "./utils";
import { deserialize, serialize } from "borsh";

const AMM_CONTRACT_SRC =
  "../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/amm";
const bytecode = fs.readFileSync(AMM_CONTRACT_SRC);
// 2 account
const privateKey =
  "afdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890";
const signer = new ethers.SigningKey("0x" + privateKey);
const owner = new ethers.Wallet(signer, ethers.provider);

const privateKey1 =
  "af1a53abf88f4821840a2934f3facfc8b1827cccd7f2e331375d2faf8c1032d2";
const signer1 = new ethers.SigningKey("0x" + privateKey1);
const user = new ethers.Wallet(signer1, ethers.provider);

async function main() {
  const chainId = parseInt(await network.provider.send("eth_chainId"));

  console.log("Chain ID:", chainId);

  // !You need to create user with this alias first
  console.log("\nCreating user...");
  await create_account(`user1.multivm`, owner.address);
  await create_account(`user2.multivm`, user.address);

  console.log("User created!");
  console.log(`user1.multivm`, owner.address);
  console.log(`user2.multivm`, user.address);

  console.log("\nDeploying tokens...");
  const TOKEN = await ethers.getContractFactory("Token");

  const token1 = await TOKEN.connect(owner).deploy(
    "Token1",
    "TKN1",
    parseEther(String(1_000_000))
  );
  const token1Address = await token1.getAddress();

  const token2 = await TOKEN.connect(owner).deploy(
    "Token2",
    "TKN2",
    parseEther(String(1_000_000))
  );
  const token2Address = await token2.getAddress();

  console.log("Tokens deployed:");
  console.log(token1Address);
  console.log(token2Address);

  await token1.connect(owner).approve(user.address, "100000000");
  await token2.connect(owner).approve(user.address, "100000000");
  console.log("\Approved...");

  console.log("\nDeploying AMM contract...");
  await deploy_contract("user2.multivm", privateKey1, toHexString(bytecode));

  const amm = await ethers.getContractAt(
    "AMM",
    await user.getAddress(),
    user
  );

  console.log(`AMM deployed`, await amm.getAddress());

  console.log(`\n —— [init] send transaction...`);

  const initTx = await user.sendTransaction({
    to: amm,
    data: serialize(transactionSchema, {
      method: "init",
      args: serialize(initArgs, []),
      gas: BigInt(300_000),
      deposit: BigInt(0),
    }),
  });
  // console.dir(initTx);
  console.log(` —— [init] ready!`);

  console.log(`\n —— [add pool] send transaction...`);
  const addPoolTx = await user.sendTransaction({
    to: amm,
    data: serialize(transactionSchema, {
      method: "add_pool",
      args: serialize(addPoolArgs, {
        token0: token1Address,
        token1: token2Address,
      }),
      gas: BigInt(300_000),
      deposit: BigInt(0),
    }),
  });

  console.log(await token1.balanceOf(owner.address), await token2.balanceOf(owner.address));
  console.log(await token1.balanceOf(user.address), await token2.balanceOf(user.address));

  const addLiquidity = await owner.sendTransaction({
    to: amm,
    data: serialize(transactionSchema, {
      method: "add_liquidity",
      args: serialize({
            struct: {
              pool_id: "u128",
              amount0: "u128",
              amount1: "u128",
            }
        }, {
        pool_id: BigInt(0),
        amount0: BigInt(300_000),
        amount1: BigInt(500_000),
      }),
      gas: BigInt(300_000),
      deposit: BigInt(0),
    }),
  });

  console.log(await token1.balanceOf(owner.address), await token2.balanceOf(owner.address));
  console.log(await token1.balanceOf(user.address), await token2.balanceOf(user.address));

  const swap = await owner.sendTransaction({
    to: amm,
    data: serialize(transactionSchema, {
      method: "swap",
      args: serialize({
          struct: {
            pool_id: "u128",
            amount0_in: "u128",
            amount1_in: "u128",
          }
      }, {
        pool_id: BigInt(0),
        amount0_in: BigInt(1_000),
        amount1_in: BigInt(0),
      }),
      gas: BigInt(300_000),
      deposit: BigInt(0),
    }),
  });

  console.log(await token1.balanceOf(owner.address), await token2.balanceOf(owner.address));
  console.log(await token1.balanceOf(user.address), await token2.balanceOf(user.address));

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
