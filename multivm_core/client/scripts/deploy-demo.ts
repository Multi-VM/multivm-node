import fs from "fs";
import { ethers } from "hardhat";
import {
  deployTokenContract,
  deployWrappedContract,
  depositWrapped,
  createUserSafe,
  deployRustAMMContract,
  rustAmmContractInit,
  rustAmmContractAddPool,
  rustAmmContractAddLiquidity,
  deploySvmAMMContract,
  svmAmmContractInit,
  svmAmmContractAddPool,
  svmAmmContractAddLiquidity,
} from "./helpers";
import { parseEther } from "ethers";

const rustAmmByteCode = fs.readFileSync("../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/amm");
const svmAmmByteCode = fs.readFileSync("../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/solana_amm");
const rustAmmPrivateKey = "d7607486f86dd992de52240a4874c4a9a49fdcefab77897044ec7da3498993b5";
const svmAmmPrivateKey = "afdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890";
const rustAmm = new ethers.Wallet(new ethers.SigningKey("0x" + rustAmmPrivateKey), ethers.provider);
const svmAmm = new ethers.Wallet(new ethers.SigningKey("0x" + svmAmmPrivateKey), ethers.provider);

async function main() {
  // Users
  console.info("Creating users...");
  const signers = await ethers.getSigners();
  const owner = signers[0];
  await createUserSafe("owner.multivm", owner.address);
  await createUserSafe("rust_amm.multivm", rustAmm.address);
  await createUserSafe("solana_amm.multivm", svmAmm.address);

  // WMVM
  console.info("\nDeploy WMVM...");
  const wToken = await deployWrappedContract(owner);
  await depositWrapped(wToken, owner, "500");
  const wTokenAddress = await wToken.getAddress();

  // Tokens
  console.info("\nDeploy 3 tokens...");
  const token1 = await deployTokenContract({ name: "Alpha", symbol: "ALPHA", signer: owner });
  const token2 = await deployTokenContract({ name: "Bravo", symbol: "BRAVO", signer: owner });
  const token3 = await deployTokenContract({ name: "Charlie", symbol: "CHARLIE", signer: owner });

  const token1Address = await token1.getAddress();
  const token2Address = await token2.getAddress();
  const token3Address = await token3.getAddress();

  console.table([
    { name: "Alpha", address: token1Address },
    { name: "Bravo", address: token2Address },
    { name: "Charlie", address: token3Address },
  ]);

  // Rust
  console.info("\nDeploy Rust AMM...");
  const rustAmmContract = await deployRustAMMContract({
    mvmAddress: "rust_amm.multivm",
    privateKey: rustAmmPrivateKey,
    byteCode: rustAmmByteCode,
    address: rustAmm.address,
    signer: rustAmm,
  });

  console.info("Setup Rust AMM...");
  await wToken.approve(rustAmm.address, parseEther(String(5_00)));
  await token1.approve(rustAmm.address, parseEther(String(1_000_000_000)));
  await token2.approve(rustAmm.address, parseEther(String(1_000_000_000)));

  await rustAmmContractInit({ contract: rustAmmContract, signer: owner });
  await rustAmmContractAddPool({ token0: wTokenAddress, token1: token1Address, contract: rustAmmContract, signer: owner });
  await rustAmmContractAddPool({ token0: token1Address, token1: token2Address, contract: rustAmmContract, signer: owner });
  await rustAmmContractAddLiquidity({ poolId: 0, amount0: 500, amount1: 1_000, contract: rustAmmContract, signer: owner });
  await rustAmmContractAddLiquidity({ poolId: 1, amount0: 1_000, amount1: 200_000, contract: rustAmmContract, signer: owner });

  // SVM
  console.info("\nDeploy SVM AMM...");
  const svmAmmContract = await deploySvmAMMContract({
    mvmAddress: "solana_amm.multivm",
    privateKey: svmAmmPrivateKey,
    byteCode: svmAmmByteCode,
    address: svmAmm.address,
    signer: svmAmm,
  });

  console.info("Setup SVM AMM...");
  await token2.approve(svmAmm.address, parseEther(String(1_000_000_000)));
  await token3.approve(svmAmm.address, parseEther(String(1_000_000_000)));

  await svmAmmContractInit({ contract: svmAmmContract, signer: owner });
  await svmAmmContractAddPool({ token0: token2Address, token1: token3Address, contract: svmAmmContract, signer: owner });
  await svmAmmContractAddLiquidity({ poolId: 1, amount0: 500, amount1: 1_000, contract: svmAmmContract, signer: owner });

  // Done
  console.info("\nDone!");
  console.table([
    { contract: "WMVM_TOKEN_ADDRESS", address: wTokenAddress },
    { contract: "RUST_AMM_ADDRESS", address: rustAmm.address },
    { contract: "SVM_AMM_ADDRESS", address: svmAmm.address },
  ]);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
