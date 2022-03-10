# 🚀 SCRT SIBYL - Contract Development Helper

![scrt sibyl image](../images/logo_horizontal.png)

This is a simple node module intended to help with development on the SECRET Network.

### Requirements

- Yarn
- Node
- A compiled WASM smart contract

### 1. Install Dependencies

run `yarn install` in the cli folder

### 2. Generate keypairs

in the same folder run `yarn keypair`

This will generate a mnemonic + address and write it to a file called keys.json. Don't worry, these are just testnet keys.

### 3. Add testnet funds to your keypair

Visit the faucent to get tokens: https://faucet.secrettestnet.io - enter your address from the keys.json file and request tokens.

### 4. Verify the max_size and your compiled wasm contract path

```sh
const MAX_SIZE = 1000;
const WASM = fs.readFileSync("../contract.wasm");
```

## 4. Upload and Initiate your contract

Run `yarn go` to upload and initiate your contract

Coming soon: query & handle functions
