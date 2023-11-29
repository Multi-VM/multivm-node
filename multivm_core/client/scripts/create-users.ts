import { ethers, network } from "hardhat";
import { create_account, get_balance } from "./utils";
import { formatEther } from "ethers";

async function main() {
  const chainId = parseInt(await network.provider.send("eth_chainId"));
  const signers = await ethers.getSigners();

  console.log("Chain ID:", chainId);
  for (const [index, signer] of signers.entries()) {
    console.log(`User ${index}:`, await signer.getAddress());
  }

  console.log("\nCreating evm accounts...");
  for (const [index, signer] of signers.entries()) {
    let signerAadress = await signer.getAddress();
    try {
      await create_account(`user${index}.multivm`, signerAadress);
      console.log(`user${index}.multivm: ${signerAadress} created!`);
    } catch (error) {
      console.log(
        `Error creating user${index}.multivm: ${signerAadress} account!`
      );
    }
  }

  console.log("\nNative balances");
  for (const signer of signers) {
    let signerAadress = await signer.getAddress();
    const mvmBalance = await get_balance(signerAadress);
    console.log(`${signerAadress}: ${formatEther(mvmBalance)} MVM`);
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
