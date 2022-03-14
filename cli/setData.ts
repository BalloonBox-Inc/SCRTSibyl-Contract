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
require("dotenv").config();

const log = console.log;

const MAX_SIZE = 1000;
const KEYPAIR = require("./keys.json");
const CONTRACT_DATA = require("./contract.json");
const WASM = fs.readFileSync("./contract.wasm");

const main = async () => {
  if (KEYPAIR?.mnemonic) {
    const signingPen = await Secp256k1Pen.fromMnemonic(KEYPAIR.mnemonic);
    const pubkey = encodeSecp256k1Pubkey(signingPen.pubkey);
    const accAddress = pubkeyToAddress(pubkey, "secret");
    const txEncryptionSeed = EnigmaUtils.GenerateNewSeed();

    const client = new SigningCosmWasmClient(
      TESTNET_URL,
      accAddress,
      (signBytes) => signingPen.sign(signBytes),
      txEncryptionSeed,
      CUSTOM_FEES
    );

    const submitScoreMsg = {
      record: {
        score: 400,
        description: "This is a description of the user score.",
      },
    };
    // const handleMsg = {increment: {}};
    // const response = await client.execute(contractId, handleMsg);
    let handle_response = await client.execute(
      CONTRACT_DATA.contractAddress,
      submitScoreMsg
    );

    const strRes = Buffer.from(handle_response.data.buffer).toString();
    if (strRes.includes("Score recorded")) {
      log(chalk.green.bold("Score Submission Successful!"));
    }
  }
};

main().catch((err) => {
  console.error(err);
});
