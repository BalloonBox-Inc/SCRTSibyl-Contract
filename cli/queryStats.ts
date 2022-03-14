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

    const queryStatsMsg = { get_stats: {} };

    let query_response = await client.queryContractSmart(
      CONTRACT_DATA.contractAddress,
      queryStatsMsg
    );

    log(chalk.green.bold("Contract Stats:"));
    log(chalk.green("Score Count: ", query_response.score_count));
    log(chalk.green("Max Size: ", query_response.max_size));
  }
};

main().catch((err) => {
  console.error(err);
});
