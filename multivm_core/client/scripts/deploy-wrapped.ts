import { formatEther, parseEther } from "ethers";
import { ethers, network } from "hardhat";
import { get_balance } from "./utils";

async function main() {
  const chainId = parseInt(await network.provider.send("eth_chainId"));
  const signers = await ethers.getSigners();

  console.log("Chain ID:", chainId);
  for (const [index, signer] of signers.entries()) {
    console.log(`User ${index}:`, await signer.getAddress());
  }

  console.log(`\nDeploying Wrapped MVM...`);
  const WRAPPED_TOKEN_CONTRACT = await ethers.getContractFactory("WMVM");
  const wToken = await WRAPPED_TOKEN_CONTRACT.deploy();

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
  for (const signer of signers) {
    const address = await signer.getAddress();
    const signed = await wToken.connect(signer);
    await signed.approve(address, parseEther("1"));
    await signed.deposit({ value: parseEther("1") });
    const mvmBalance = await get_balance(address);
    console.log(`${address}: ${formatEther(mvmBalance)} MVM`);
    const balance = await wToken.balanceOf(address);
    console.log(`${address}: ${balance}`);
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
