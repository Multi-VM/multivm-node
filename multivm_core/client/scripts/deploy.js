async function main() {
  const [user1, user2] = await ethers.getSigners();

  const privateKey1 = "0x" + "b0c4d20fdb2eb44488a28a5e99020a05cb8ccf98c37819bafd219e84b4023ce4";
  const signingKey1 = new ethers.SigningKey(privateKey1);
  const user_addr1 = ethers.computeAddress(privateKey1);

  const privateKey2 = "0x" + "afdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890";
  const signingKey2 = new ethers.SigningKey(privateKey2);
  const user_addr2 = ethers.computeAddress(privateKey2);

  var token;
  const create_accounts = 1;
  // const create_accounts = 0;

  if (create_accounts) {
    console.log("Creating evm accounts...");
    create_account("user1.multivm", signingKey1.publicKey);
    create_account("user2.multivm", signingKey2.publicKey);
    token = await ethers.deployContract("Token");
  } else {
    token = await ethers.getContractAt("Token", "0x1029E07C605e94E2Ae3c6E6E4B8B47c959696545");
    await token.connect(user1).transfer(user_addr2, "3000");
  }

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