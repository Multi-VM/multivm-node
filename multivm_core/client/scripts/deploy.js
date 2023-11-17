async function main() {
  const [user1, user2] = await ethers.getSigners();

  const privateKey1 = "0x" + "af1a53abf88f4821840a2934f3facfc8b1827cccd7f2e331375d2faf8c1032d2";
  const signingKey1 = new ethers.SigningKey(privateKey1);
  const user_addr1 = ethers.computeAddress(privateKey1);

  const privateKey2 = "0x" + "afdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890";
  const signingKey2 = new ethers.SigningKey(privateKey2);
  const user_addr2 = ethers.computeAddress(privateKey2);

  console.log(user_addr1, user_addr2);
  return;

  var token;
  let create_accounts = 1;
  // create_accounts = 0;

  // if (create_accounts) {
    console.log("Creating evm accounts...");
    create_account("user1.multivm", signingKey1.publicKey);
    create_account("user2.multivm", signingKey2.publicKey);
    // token = await ethers.deployContract("Token");
  // } else {
    const fs = require('fs');
    const bytecode = fs.readFileSync("/Users/nikita/Develop/spin-node-wip/example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/token_contract");
    deploy_contract("user1.multivm", toHexString(bytecode));

    console.log("Calling token...");

    token = await ethers.getContractAt("AMM", user_addr1);
    let input = await token.connect(user1).init("my string");
    console.log(input);
    return;

  //   return;

  //   token = await ethers.getContractAt("Token", "0x1029E07C605e94E2Ae3c6E6E4B8B47c959696545");
  //   await token.connect(user1).transfer(user_addr2, "3000");
  // }

  return;

  // const balance1 = await get_balance(user_addr1);
  // console.log(balance1);
  // const balance2 = await get_balance(user_addr2);
  // console.log(balance2);
  // return;


  console.log("Token address:", await token.getAddress());
  console.log("Total supply:", await token.totalSupply());

  if (create_accounts) {
    await token.connect(user1).transfer(user_addr2, "3000");
  }

  console.log(user1.address, ": ", await token.balanceOf(user1.address));
  console.log(user2.address, ": ", await token.balanceOf(user2.address));
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });

function call(method, params) {
  var request = require('sync-request');
  const response = request('POST', 'http://127.0.0.1:8080/', {
    headers: {
      'Content-Type': 'application/json',
    },
    json: {
      "jsonrpc": "2.0",
      "method": method,
      "params": params,
      "id": 1
    }
  });
  return JSON.parse(response.body.toString()).result;
}

function create_account(mvm, publicKey) {
  return call("mvm_createAccount", [{
    "multivm": mvm,
    "evm": publicKey
  }]);
}

function get_balance(address) {
  return call("eth_getBalance", [address]);
}

function deploy_contract(mvm, bytecode) {
  return call("mvm_deployContract", [{
    "multivm": mvm,
    "bytecode": bytecode,
  }]);
}

function toHexString(byteArray) {
  return Array.from(byteArray, function(byte) {
    return ('0' + (byte & 0xFF).toString(16)).slice(-2);
  }).join('')
}