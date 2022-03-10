const {
  EnigmaUtils,
  Secp256k1Pen,
  SigningCosmWasmClient,
  pubkeyToAddress,
  encodeSecp256k1Pubkey,
} = require("secretjs");
const fs = require("fs");

// Load environment variables
require("dotenv").config();

const MAX_SIZE = 1000;
const KEYPAIR = require("./keys.json");
const WASM = fs.readFileSync("../contract.wasm");
const TESTNET_URL = "http://testnet.securesecrets.org:1317/";

const customFees = {
  upload: {
    amount: [{ amount: "5000000", denom: "uscrt" }],
    gas: "5000000",
  },
  init: {
    amount: [{ amount: "500000", denom: "uscrt" }],
    gas: "500000",
  },
  exec: {
    amount: [{ amount: "500000", denom: "uscrt" }],
    gas: "500000",
  },
  send: {
    amount: [{ amount: "80000", denom: "uscrt" }],
    gas: "80000",
  },
};

const main = async () => {
  if (KEYPAIR?.mnemonic) {
    const signingPen = await Secp256k1Pen.fromMnemonic(KEYPAIR.mnemonic).catch(
      (err: any) => {
        throw new Error(`Could not get signing pen: ${err}`);
      }
    );

    // Get the public key
    const pubkey = encodeSecp256k1Pubkey(signingPen.pubkey);

    console.log({ pubkey });

    // get the wallet address
    const accAddress = pubkeyToAddress(pubkey, "secret");

    const txEncryptionSeed = EnigmaUtils.GenerateNewSeed();

    const client = new SigningCosmWasmClient(
      TESTNET_URL,
      accAddress,
      (signBytes) => signingPen.sign(signBytes),
      txEncryptionSeed,
      customFees
    );

    const wasm = fs.readFileSync("../contract.wasm");
    const uploadReceipt = await client.upload(wasm, {}).catch((err) => {
      throw new Error(`Could not upload contract: ${err}`);
    });

    console.log("Received upload receipt, instantiating contract");

    // Get the code ID from the receipt
    const { codeId } = uploadReceipt;

    const initMsg = { max_size: MAX_SIZE };
    const contract = await client
      .instantiate(codeId, initMsg, accAddress.slice(6))
      .catch((err) => {
        throw new Error(`Could not instantiate contract: ${err}`);
      });

    const { contractAddress } = contract;

    console.log("contract: ", contract, "address:", contractAddress);
  }
};

main().catch((err) => {
  console.error(err);
});
