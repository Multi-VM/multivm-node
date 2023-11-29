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
import { serialize } from "borsh";

const AMM_CONTRACT_SRC =
  "../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/amm";
const bytecode = fs.readFileSync(AMM_CONTRACT_SRC);
// 2 account
const privateKey =
  "afdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890";
const signer = new ethers.SigningKey("0x" + privateKey);
const owner = new ethers.Wallet(signer, ethers.provider);

async function main() {
  const chainId = parseInt(await network.provider.send("eth_chainId"));

  console.log("Chain ID:", chainId);

  // !You need to create user with this alias first
  console.log("\nCreating user...");
  await create_account(`user1.multivm`, owner.address);

  console.log("User created!");
  console.log(`user1.multivm`, owner.address);

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

  console.log("\nDeploying AMM contract...");
  await deploy_contract("user1.multivm", privateKey, toHexString(bytecode));

  const amm = await ethers.getContractAt(
    "AMM",
    await owner.getAddress(),
    owner
  );

  console.log(`AMM deployed`, await amm.getAddress());

  console.log(`\n —— [init] send transaction...`);

  const initTx = await owner.sendTransaction({
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
  const addPoolTx = await owner.sendTransaction({
    to: amm,
    data: serialize(transactionSchema, {
      method: "init",
      args: serialize(addPoolArgs, {
        token0: token1Address,
        token1: token2Address,
      }),
      gas: BigInt(300_000),
      deposit: BigInt(0),
    }),
  });
  const addPoolTx2 = await owner.sendTransaction({
    to: amm,
    data: serialize(transactionSchema, {
      method: "init",
      args: serialize(addPoolArgs, {
        token0: token2Address,
        token1: token1Address,
      }),
      gas: BigInt(300_000),
      deposit: BigInt(0),
    }),
  });
  // console.dir(addPoolTx);
  console.log(` —— [add pool] ready!`);

  // read pool
  try {
    const pools = await view("user1.multivm", "get_pools", []);
    console.log(pools);
  } catch (error) {
    console.error(error);
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
