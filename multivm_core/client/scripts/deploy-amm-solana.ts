import fs from "fs";
import { ethers, network } from "hardhat";
import { parseEther } from "ethers";
import * as solanaWeb3 from "@solana/web3.js";
import {
  addPoolArgs,
  SolanaContextSchema,
  create_account,
  deploy_contract,
  solana_data,
  initArgs,
  toHexString,
  transactionSchema,
  view,
} from "./utils";
import { serialize } from "borsh";

const AMM_CONTRACT_SRC =
  "../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/solana_amm";
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
  console.log("\nCreating solana amm account...");
  await create_account(`solana_amm.multivm`, owner.address);

  console.log("Solana amm contract created!");
  console.log(`solana_amm.multivm`, owner.address);

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

  console.log("\nDeploying Solana AMM contract...");
  await deploy_contract("solana_amm.multivm", "svm", privateKey, toHexString(bytecode));

  const amm = await ethers.getContractAt(
    "AMM",
    await owner.getAddress(),
    owner
  );

  console.log(`AMM deployed`, await amm.getAddress());

  console.log(`\n —— [init] send transaction...`);

  const program_id = new solanaWeb3.PublicKey("SzKDof1CX8t39ns784mNZtq6mCs2NT9p7B2KBNmFYPR");

  const token1Seed = Uint8Array.from(Buffer.from(token1Address.slice(2), 'hex'));
  const token2Seed = Uint8Array.from(Buffer.from(token2Address.slice(2), 'hex'));


  const [state_account_address, _] = solanaWeb3.PublicKey.findProgramAddressSync([Buffer.from("state")], program_id);
  // const [pool_account_address, _] = solanaWeb3.PublicKey.findProgramAddressSync([token1Seed, token2Seed], program_id);

  // console.log(`pool_account_address: ${pool_account_address.toBase58()}`);

  let data = await solana_data(program_id.toBase58(), state_account_address.toBase58());
  console.log(data)

  const instruction_data = new Uint8Array([0]);
  // instruction_data.set([1]);
  // instruction_data.set(token1Seed, 1);
  // instruction_data.set(token2Seed, 1 + token1Seed.length);

  // console.log(`token1Seed`, token1Seed);

  const init = await owner.sendTransaction({
    to: amm,
    data: serialize(transactionSchema, {
      method: "init",
      args: serialize(SolanaContextSchema, {
        accounts: [state_account_address.toBytes(), state_account_address.toBytes()],
        instruction_data: instruction_data,
      }),
      gas: BigInt(300_000),
      deposit: BigInt(0),
    }),
  });
  console.dir(init);

  let data2 = await solana_data(program_id.toBase58(), state_account_address.toBase58());
  console.log(data2)

  console.log(` ——[init] ready!`);

}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
