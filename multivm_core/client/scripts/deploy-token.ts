import { parseEther, formatUnits } from "ethers";
import { ethers } from "hardhat";
import { createUsersFromSigners, deployTokenContract } from "./helpers";

// change this
const TOKEN_NAME = "ERC20 My Token";
const TOKEN_SYMBOL = "TKN";
const TOKEN_SUPPLY = parseEther(String(1_000_000_000));

async function main() {
  await createUsersFromSigners();
  console.log(`\nDeploying token...`);
  const TOKEN = await deployTokenContract({
    name: TOKEN_NAME,
    symbol: TOKEN_SYMBOL,
    supply: TOKEN_SUPPLY,
  });

  const tokenName = await TOKEN.name();
  const tokenSymbol = await TOKEN.symbol();
  const tokenDecimals = await TOKEN.decimals();
  const tokenAddress = await TOKEN.getAddress();
  const tokenSupply = await TOKEN.totalSupply();

  console.log(`\nToken deployed!`);
  console.log(`Name: ${tokenName}`);
  console.log(`Symbol: ${tokenSymbol}`);
  console.log(`Decimals: ${tokenDecimals.toString()}`);
  console.log(`Address: ${tokenAddress}`);
  console.log(`Supply: ${formatUnits(tokenSupply, tokenDecimals)}`);

  console.log(`\nToken transfer to all signers...`);
  const signers = await ethers.getSigners();
  const owner = await TOKEN.connect(signers[0]);
  for (const signer of signers.slice(1)) {
    const address = await signer.getAddress();
    await owner.transfer(address, parseEther(String(1_000_000)));
  }

  for (const signer of signers) {
    const address = await signer.getAddress();
    const balance = await TOKEN.balanceOf(address);
    console.log(`${address}: ${formatUnits(balance, tokenDecimals)}`);
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
