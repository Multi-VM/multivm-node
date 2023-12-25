import fs from "fs";
import { ethers } from "hardhat";
import { deployTokenContract, deployWrappedContract, deployAMMContract, depositWrapped, createUserSafe, ammContractInit, ammContractAddPool, ammContractAddLiquidity } from "./helpers";
import { parseEther } from "ethers";

const ammByteCode = fs.readFileSync("../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/amm");
const ammPrivateKey = "d7607486f86dd992de52240a4874c4a9a49fdcefab77897044ec7da3498993b5";
const amm = new ethers.Wallet(new ethers.SigningKey("0x" + ammPrivateKey), ethers.provider);

async function main() {
  const signers = await ethers.getSigners();
  const owner = signers[0];
  await createUserSafe("owner.multivm", owner.address);
  await createUserSafe("amm.multivm", amm.address);

  const wToken = await deployWrappedContract(owner);
  await depositWrapped(wToken, owner, "500");

  const wTokenAddress = await wToken.getAddress();
  const token1 = await deployTokenContract({ name: "Token 1", symbol: "TKN1", signer: owner });
  const token2 = await deployTokenContract({ name: "Token 2", symbol: "TKN2", signer: owner });
  const token1Address = await token1.getAddress();
  const token2Address = await token2.getAddress();

  const ammContract = await deployAMMContract({
    mvmAddress: "amm.multivm",
    privateKey: ammPrivateKey,
    byteCode: ammByteCode,
    address: amm.address,
    signer: amm,
  });

  await token1.approve(amm.address, parseEther(String(1_000_000_000)));
  await token2.approve(amm.address, parseEther(String(1_000_000_000)));

  await ammContractInit({ contract: ammContract, signer: owner });
  await ammContractAddPool({ token0: wTokenAddress, token1: token1Address, contract: ammContract, signer: owner });
  await ammContractAddPool({ token0: token1Address, token1: token2Address, contract: ammContract, signer: owner });
  await ammContractAddLiquidity({ poolId: 0, amount0: 500, amount1: 1_000, contract: ammContract, signer: owner });
  await ammContractAddLiquidity({ poolId: 1, amount0: 1_000, amount1: 200_000, contract: ammContract, signer: owner });

  console.log("WMWM:", wTokenAddress);
  console.log("AMM:", amm.address);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
