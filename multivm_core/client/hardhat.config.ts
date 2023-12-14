import dotenv from "dotenv";
import "@nomicfoundation/hardhat-toolbox";
import "@nomicfoundation/hardhat-ethers";
import "@nomicfoundation/hardhat-chai-matchers";
import "@ethersproject/address";

dotenv.config();

/** @type import('hardhat/config').HardhatUserConfig */
module.exports = {
  solidity: {
    version: "0.8.20",
    settings: {
      optimizer: {
        enabled: true,
        runs: 1000,
      },
    },
  },
  networks: {
    multivm: {
      url: process.env.MVM_RPC_URL || "http://127.0.0.1:8080",
      accounts: ["af1a53abf88f4821840a2934f3facfc8b1827cccd7f2e331375d2faf8c1032d2", "afdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890"],
    },
    goerli: {
      url: "https://eth-goerli.public.blastapi.io",
      accounts: ["d7607486f86dd992de52240a4874c4a9a49fdcefab77897044ec7da3498993b5"],
    },
  },
  gasReporter: {
    enabled: process.env.REPORT_GAS !== undefined,
    currency: "USD",
  },
  paths: {
    tests: "./test",
  },
};
