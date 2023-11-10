require("@nomicfoundation/hardhat-toolbox");
require('@ethersproject/address');

/** @type import('hardhat/config').HardhatUserConfig */
module.exports = {
  solidity: "0.8.20",
  networks: {
    multivm: {
      url: "http://127.0.0.1:8080",
      accounts: [
        "b0c4d20fdb2eb44488a28a5e99020a05cb8ccf98c37819bafd219e84b4023ce4",
        "afdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890"
      ]
    }
  }
};
