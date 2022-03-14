import { CUSTOM_FEES, TESTNET_URL } from "./CONSTANTS";

const {
  EnigmaUtils,
  Secp256k1Pen,
  SigningCosmWasmClient,
  pubkeyToAddress,
  encodeSecp256k1Pubkey,
} = require("secretjs");
const fs = require("fs");
const chalk = require("chalk");

const log = console.log;
// Load environment variables
require("dotenv").config();

const MAX_SIZE = 1000;
const KEYPAIR = require("./keys.json");
const WASM = fs.readFileSync("./contract.wasm");

const main = async () => {
  if (KEYPAIR?.mnemonic) {
    const signingPen = await Secp256k1Pen.fromMnemonic(KEYPAIR.mnemonic).catch(
      (err: any) => {
        throw new Error(`Could not get signing pen: ${err}`);
      }
    );

    // Get the public key
    const pubkey = encodeSecp256k1Pubkey(signingPen.pubkey);

    // get the wallet address
    const accAddress = pubkeyToAddress(pubkey, "secret");

    const txEncryptionSeed = EnigmaUtils.GenerateNewSeed();

    const client = new SigningCosmWasmClient(
      TESTNET_URL,
      accAddress,
      (signBytes) => signingPen.sign(signBytes),
      txEncryptionSeed,
      CUSTOM_FEES
    );

    const uploadReceipt = await client.upload(WASM, {}).catch((err) => {
      throw new Error(`Could not upload contract: ${err}`);
    });

    log(chalk.green("Received upload receipt, instantiating contract"));

    // Get the code ID from the receipt
    const { codeId } = uploadReceipt;

    const initMsg = { max_size: MAX_SIZE, prng_seed: "seed" };
    const contract = await client
      .instantiate(codeId, initMsg, accAddress.slice(6))
      .catch((err) => {
        throw new Error(`Could not instantiate contract: ${err}`);
      });

    const { contractAddress } = contract;

    fs.writeFileSync(
      "contract.json",
      JSON.stringify({
        contractAddress,
        transactionHash: contract.transactionHash,
        initData: contract.data,
      })
    );
  }
};

main().catch((err) => {
  console.error(err);
});
