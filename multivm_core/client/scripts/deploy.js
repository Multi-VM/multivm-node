async function main() {
  const fs = require("fs");
  const borsh = require("borsh");

  const [user1, user2] = await ethers.getSigners();

  const privateKey1 = "af1a53abf88f4821840a2934f3facfc8b1827cccd7f2e331375d2faf8c1032d2";
  const signingKey1 = new ethers.SigningKey("0x" + privateKey1);
  const user_addr1 = ethers.computeAddress("0x" + privateKey1);

  const privateKey2 = "afdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890";
  const signingKey2 = new ethers.SigningKey("0x" + privateKey2);
  const user_addr2 = ethers.computeAddress("0x" + privateKey2);

  console.log("Creating evm accounts...");
  create_account("user1.multivm", signingKey1.publicKey);
  create_account("user2.multivm", signingKey2.publicKey);
  const token1 = await ethers.deployContract("Token");
  const token2 = await ethers.deployContract("Token");
  console.log("Tokens:", token1.target, token2.target);

  const bytecode = fs.readFileSync("../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/amm");
  deploy_contract("user1.multivm", toHexString(bytecode));

  console.log("Calling amm...");

  const amm = await ethers.getContractAt("AMM", user_addr1);
  const signer = new ethers.Wallet(privateKey1, ethers.provider);  
  const schema = {
    struct: {
      method: "string",
      args: {
        array: { type: "u8" }
      },
      gas: "u64",
      deposit: "u128"
    }
  };

  // const empty_args_data = borsh.serialize({ array: { type: 'u8' }}, []);
  // var data = borsh.serialize(schema, { 
  //   method: "init",
  //   args: empty_args_data,
  //   gas: BigInt(300000),
  //   deposit: BigInt(0)
  // });
  // await signer.sendTransaction({
  //   to: amm,
  //   data: data
  // });

  const args_schema = {
    struct: {
      token0: "string",
      token1: "string"
    }
  };
  const args_data = borsh.serialize(args_schema, {
      token0: token1.target,//.replace("0x", ""),
      token1: token2.target//.replace("0x", "")
    }
  );
  data = borsh.serialize(schema, { 
    method: "add_pool",
    args: args_data,
    gas: BigInt(300000),
    deposit: BigInt(0)
  });
  await signer.sendTransaction({
    to: amm,
    data: data
  });


  
  return;

  // console.log("Token address:", await token.getAddress());
  // console.log("Total supply:", await token.totalSupply());
  // const balance1 = await get_balance(user_addr1);
  // console.log(balance1);
  // const balance2 = await get_balance(user_addr2);
  // console.log(balance2);
  // await token.connect(user1).transfer(user_addr2, "3000");
  // console.log(user1.address, ": ", await token.balanceOf(user1.address));
  // console.log(user2.address, ": ", await token.balanceOf(user2.address));
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });

function call(method, params) {
  var request = require("sync-request");
  const response = request("POST", "http://127.0.0.1:8080/", {
    headers: {
      "Content-Type": "application/json",
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
    return ("0" + (byte & 0xFF).toString(16)).slice(-2);
  }).join("")
}