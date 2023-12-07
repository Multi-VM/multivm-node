import { field, string } from "@dao-xyz/borsh";

let ID = 0;
const getID = () => ++ID;

export async function call(method: string, params: any) {
  const response = await fetch("http://127.0.0.1:8080/", {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    method: "POST",
    body: JSON.stringify({
      jsonrpc: "2.0",
      method: method,
      params: params,
      id: getID(),
    }),
  });

  return await response.json();
}

export async function create_account(mvm: string, address: string) {
  return await call("mvm_debugAirdrop", [
    {
      multivm: mvm,
      address: address,
    },
  ]);
}

export async function view(account: string, method: string, args: any) {
  return await call("mvm_viewCall", [
    account,
    {
      method: method,
      args: args,
      gas: 0, // ?
      deposit: 0, // ?
    },
  ]);
}

export async function get_balance(address: string) {
  return await call("eth_getBalance", [
    address,
    // {
    //   address,
    //   "latest"
    // },
  ]).then((r) => BigInt(r.result).toString());
}

export async function deploy_contract(mvm: string, privateKey: string, bytecode: string) {
  return await call("mvm_deployContract", [
    {
      multivm: mvm,
      private_key: privateKey,
      bytecode: bytecode,
    },
  ]);
}

export const toHexString = (byteArray: Buffer) => Array.from(byteArray, (byte) => ("0" + (byte & 0xff).toString(16)).slice(-2)).join("");

export const transactionSchema = {
  struct: {
    method: "string",
    args: {
      array: { type: "u8" },
    },
    gas: "u64",
    deposit: "u128",
  },
};

export const initArgs = {
  array: { type: "u8" },
};

export const addPoolArgs = {
  struct: {
    token0: "string",
    token1: "string",
  },
};

export const addLiquidityArgs = {
  struct: {
    pool_id: "u128",
    amount0: "u128",
    amount1: "u128",
  },
};

export const swapArgs = {
  struct: {
    pool_id: "u128",
    amount0_in: "u128",
    amount1_in: "u128",
  },
};

export const PoolSchema = {
  struct: {
    id: "u128",
    token0: { struct: { symbol: "string", address: "string" } },
    token1: { struct: { symbol: "string", address: "string" } },
    reserve0: "u128",
    reserve1: "u128",
    total_shares: "u128",
  },
};

export const GetPoolSchema = {
  option: PoolSchema,
};
export const GetPoolsSchema = {
  option: PoolSchema,
};
