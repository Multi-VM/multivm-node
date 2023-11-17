require("@nomicfoundation/hardhat-toolbox");
require('@ethersproject/address');

/** @type import('hardhat/config').HardhatUserConfig */
module.exports = {
  solidity: "0.8.20",
  networks: {
    multivm: {
      url: "http://127.0.0.1:8080",
      accounts: [
        "af1a53abf88f4821840a2934f3facfc8b1827cccd7f2e331375d2faf8c1032d2",
        "afdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890"
      ]
    }
  }
};
