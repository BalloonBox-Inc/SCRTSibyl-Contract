import { Bip39, Random } from "@iov/crypto";
import { Secp256k1Pen, pubkeyToAddress, encodeSecp256k1Pubkey } from "secretjs";
const fs = require("fs");

/*
This endpoint will generate a mnemonic and secret public address. 
// visit faucet to get tokens: https://faucet.secrettestnet.io/
*/

async function main() {
  try {
    const mnemonic = Bip39.encode(Random.getBytes(16)).toString();
    const signingPen = await Secp256k1Pen.fromMnemonic(mnemonic);
    const pubkey = encodeSecp256k1Pubkey(signingPen.pubkey);
    const address = pubkeyToAddress(pubkey, "secret");

    fs.writeFileSync("keys.json", JSON.stringify({ mnemonic, address }));
    return { mnemonic, address };
  } catch (error) {
    const errorMessage =
      error instanceof Error ? error.message : "Unknown Error";
    return errorMessage;
  }
}

main()
  .then((data) => {
    console.log({ data });
    process.exit(0);
  })
  .catch((err) => {
    console.error("there was an error:", err);
    process.exit(1);
  });
