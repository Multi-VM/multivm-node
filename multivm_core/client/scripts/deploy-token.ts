import { parseEther, formatUnits } from "ethers";
import { ethers, network } from "hardhat";

// change this
const TOKEN_NAME = "ERC20 My Token";
const TOKEN_SYMBOL = "TKN";
const TOKEN_SUPPLY = parseEther(String(1_000_000_000));

async function main() {
  const chainId = parseInt(await network.provider.send("eth_chainId"));
  const signers = await ethers.getSigners();

  console.log("Chain ID:", chainId);
  for (const [index, signer] of signers.entries()) {
    console.log(`User ${index}:`, await signer.getAddress());
  }

  console.log(`\nDeploying token...`);
  const TOKEN_CONTRACT = await ethers.getContractFactory("Token");
  const TOKEN = await TOKEN_CONTRACT.deploy(
    TOKEN_NAME,
    TOKEN_SYMBOL,
    TOKEN_SUPPLY
  );

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
