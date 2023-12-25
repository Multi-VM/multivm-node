import { formatEther } from "ethers";
import { ethers } from "hardhat";
import { createUsersFromSigners, deployWrappedContract, depositWrapped } from "./helpers";

async function main() {
  await createUsersFromSigners();
  const wToken = await deployWrappedContract();
  const tokenAddress = await wToken.getAddress();
  const tokenName = await wToken.name();
  const tokenSymbol = await wToken.symbol();
  const tokenDecimals = await wToken.decimals();

  console.log(`\nToken deployed!`);
  console.log(`Name: ${tokenName}`);
  console.log(`Symbol: ${tokenSymbol}`);
  console.log(`Decimals: ${tokenDecimals.toString()}`);
  console.log(`Address: ${tokenAddress}`);

  // Test deposit
  const signers = await ethers.getSigners();
  for (const signer of signers) {
    const address = await signer.getAddress();
    await depositWrapped(wToken, signer, "500");

    const mvmBalance = await ethers.provider.getBalance(signer.address);
    console.log(`${address}: ${formatEther(mvmBalance)} MVM`);

    const balance = await wToken.balanceOf(address);
    console.log(`${address}: ${formatEther(balance)} WMVM`);
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
