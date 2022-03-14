const { CosmWasmClient } = require("secretjs");
import { Bip39, Random } from "@iov/crypto";
import { Secp256k1Pen, pubkeyToAddress, encodeSecp256k1Pubkey } from "secretjs";
import { TESTNET_URL } from "./CONSTANTS";
const fs = require("fs");
const chalk = require("chalk");

const log = console.log;

const DECIMAL_OFFSET = 10 ** 6;
/*
This endpoint will generate a mnemonic and secret public address. 
// visit faucet to get tokens: https://faucet.secrettestnet.io/
*/
const KEYPAIR = require("./keys.json");
async function main() {
  const signingPen = await Secp256k1Pen.fromMnemonic(KEYPAIR.mnemonic);
  const pubkey = encodeSecp256k1Pubkey(signingPen.pubkey);
  const address = pubkeyToAddress(pubkey, "secret");

  const client = new CosmWasmClient(TESTNET_URL);

  // Query the Account object
  const account = await client.getAccount(address);
  // Return the balance
  const amount = account?.balance[0].amount;

  return parseFloat((parseFloat(amount) / DECIMAL_OFFSET).toFixed());
}

main()
  .then((balanceAmount: any) => {
    if (balanceAmount) {
      log(chalk.blue(`Address Balance: ${balanceAmount} SCRT`));
    }
    process.exit(0);
  })
  .catch((err) => {
    console.error("there was an error:", err);
    process.exit(1);
  });
