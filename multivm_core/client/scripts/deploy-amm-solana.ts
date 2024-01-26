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
  accountInfo,
  SolanaAmmSchema,
  SolanaAmmStateSchema,
  bigintToBeBytes,
  SolanaAmmPoolSchema,
  SolanaAmmTokenSchema,
} from "./utils";
import { deserialize, serialize } from "borsh";

const AMM_CONTRACT_SRC = "../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/solana_amm";
const bytecode = fs.readFileSync(AMM_CONTRACT_SRC);
// 2 account
const privateKey = "afdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890";
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

  const token1 = await TOKEN.connect(owner).deploy("Token1", "TKN1", parseEther(String(1_000_000)));
  const token1Address = await token1.getAddress();
  token1.transfer("0x121348F398681B4d021826EB6423805df7CD25D9", parseEther(String(1_000)));

  const token2 = await TOKEN.connect(owner).deploy("Token2", "TKN2", parseEther(String(1_000_000)));
  const token2Address = await token2.getAddress();

  console.log("Tokens deployed:");
  console.log(token1Address);
  console.log(token2Address);

  console.log("\nDeploying Solana AMM contract...");
  await deploy_contract("solana_amm.multivm", "svm", privateKey, toHexString(bytecode));

  const amm = await ethers.getContractAt("AMM", await owner.getAddress(), owner);
  const ammAddress = await amm.getAddress();
  console.log(`AMM deployed`, ammAddress);

  await token1.approve(ammAddress, parseEther(String(1_000_000)));
  await token2.approve(ammAddress, parseEther(String(1_000_000)));

  const program_id = new solanaWeb3.PublicKey((await accountInfo("solana_amm.multivm"))["result"]["solana_address"]);
  const owner_solana_id = new solanaWeb3.PublicKey((await accountInfo(owner.address))["result"]["solana_address"]);

  console.log(`\n —— [init] send transaction...`);
  const [state_account_id, _] = solanaWeb3.PublicKey.findProgramAddressSync([Buffer.from("state")], program_id);
  {
    const instruction_data = new Uint8Array([0]);
    // const instruction_data = serialize(SolanaAmmSchema, {
    // });
    const init = await owner.sendTransaction({
      to: amm,
      data: serialize(transactionSchema, {
        method: "init",
        args: serialize(SolanaContextSchema, {
          accounts: [owner_solana_id.toBytes(), state_account_id.toBytes()],
          instruction_data: instruction_data,
        }),
        gas: BigInt(300_000),
        deposit: BigInt(0),
      }),
    });
    console.dir(init);
    console.log(` ——[init] ready!`);
  }

  const state_account_data = (await solana_data(program_id.toBase58(), state_account_id.toBase58()))["result"];
  const state = deserialize(SolanaAmmStateSchema, state_account_data);
  console.log(state);
  const pool_id = state["next_pool_id"];
  const pool_id_bytes = bigintToBeBytes(pool_id, 16);
  console.log(pool_id);

  const [pool_state_account_id] = solanaWeb3.PublicKey.findProgramAddressSync([Buffer.from("pool"), pool_id_bytes], program_id);

  // ADD POOL
  {
    console.log(`\n —— [add_pool] send transaction...`);

    const instruction_data = serialize(SolanaAmmSchema, {
      add_pool: {
        token0: token1Address,
        token1: token2Address,
      },
    });
    const add_pool = await owner.sendTransaction({
      to: amm,
      data: serialize(transactionSchema, {
        method: "",
        args: serialize(SolanaContextSchema, {
          accounts: [owner_solana_id.toBytes(), state_account_id.toBytes(), pool_state_account_id.toBytes()],
          instruction_data: instruction_data,
        }),
        gas: BigInt(300_000),
        deposit: BigInt(0),
      }),
    });
    console.dir(add_pool);

    const pool_state_data = (await solana_data(program_id.toBase58(), pool_state_account_id.toBase58()))["result"];
    const pool_state = deserialize(SolanaAmmPoolSchema, pool_state_data);
    console.log(pool_state);
  }
  console.log(` ——[add_pool] ready!`);

  // ADD LIQUIDITY
  console.log(`\n —— [add_liquidity] send transaction...`);
  const [user_pool_shares_account_id] = solanaWeb3.PublicKey.findProgramAddressSync([Buffer.from("user_pool_shares"), owner_solana_id.toBytes(), pool_id_bytes], program_id);
  {
    const instruction_data = serialize(SolanaAmmSchema, {
      add_liquidity: {
        amount0: parseEther(String(100)),
        amount1: parseEther(String(20_000)),
      },
    });
    const add_liquidity = await owner.sendTransaction({
      to: amm,
      data: serialize(transactionSchema, {
        method: "",
        args: serialize(SolanaContextSchema, {
          accounts: [owner_solana_id.toBytes(), pool_state_account_id.toBytes(), user_pool_shares_account_id.toBytes()],
          instruction_data: instruction_data,
        }),
        gas: BigInt(300_000),
        deposit: BigInt(0),
      }),
    });
    console.dir(add_liquidity);

    const pool_state_data = (await solana_data(program_id.toBase58(), pool_state_account_id.toBase58()))["result"];
    const pool_state = deserialize(SolanaAmmPoolSchema, pool_state_data);
    console.log(pool_state);
  }
  console.log(` ——[add_liquidity] ready!`);

  // SWAP
  console.log(`\n —— [swap] send transaction...`);
  {
    const instruction_data = serialize(SolanaAmmSchema, {
      swap: {
        amount0_in: parseEther(String(1)),
        amount1_in: 0,
      },
    });
    const swap = await owner.sendTransaction({
      to: amm,
      data: serialize(transactionSchema, {
        method: "",
        args: serialize(SolanaContextSchema, {
          accounts: [owner_solana_id.toBytes(), pool_state_account_id.toBytes()],
          instruction_data: instruction_data,
        }),
        gas: BigInt(300_000),
        deposit: BigInt(0),
      }),
    });
    console.dir(swap);

    const pool_state_data = (await solana_data(program_id.toBase58(), pool_state_account_id.toBase58()))["result"];
    const pool_state = deserialize(SolanaAmmPoolSchema, pool_state_data);
    console.log(pool_state);
  }
  console.log(` —[swap] ready!`);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
