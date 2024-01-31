CREATE TABLE blocks (
  id INTEGER PRIMARY KEY NOT NULL,
  number INTEGER UNIQUE NOT NULL,
  hash TEXT NOT NULL,
  timestamp INTEGER NOT NULL,
  txs_count INTEGER NOT NULL
);

CREATE TABLE accounts (
  id INTEGER PRIMARY KEY,
  fvm_address TEXT,
  evm_address TEXT NOT NULL,
  svm_address TEXT NOT NULL, 
  created_at_block_id INTEGER NOT NULL,
  modified_at_block_id INTEGER NOT NULL,
  executable_type TEXT,
  native_balance TEXT NOT NULL
);

CREATE TABLE stats (
  id INTEGER PRIMARY KEY,
  timestamp INTEGER NOT NULL,
  block_id INTEGER NOT NULL,
  total_txs INTEGER NOT NULL,
  total_accounts INTEGER NOT NULL,
  total_contracts INTEGER NOT NULL
);

CREATE TABLE transactions (
  id INTEGER PRIMARY KEY NOT NULL,
  hash TEXT UNIQUE NOT NULL,
  block_id INTEGER NOT NULL,
  signer_account_id INTEGER NOT NULL,
  receiver_account_id INTEGER NOT NULL,
  format TEXT NOT NULL,
  nonce INTEGER NOT NULL
);

CREATE TABLE receipts (
  id INTEGER PRIMARY KEY,
  transaction_id INTEGER NOT NULL,
  parent_receipt_id INTEGER,
  index_in_transaction INTEGER NOT NULL,
  result BOOLEAN NOT NULL,
  response TEXT,
  gas_used INTEGER NOT NULL,
  contract_account_id INTEGER NOT NULL,
  call_method TEXT NOT NULL,
  call_args TEXT NOT NULL,
  call_gas INTEGER NOT NULL,
  call_deposit TEXT NOT NULL
);

CREATE TABLE events (
  id INTEGER PRIMARY KEY,
  receipt_id INTEGER NOT NULL,
  index_in_receipt INTEGER NOT NULL,
  message TEXT NOT NULL
);


-- System Meta Contract
INSERT INTO accounts (fvm_address, evm_address, svm_address, created_at_block_id, modified_at_block_id, executable_type, native_balance)
VALUES ("multivm", "0000000000000000000000000000000000000000", "multivm", 1, 1, "FVM", 0);
