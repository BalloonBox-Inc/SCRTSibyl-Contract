export const TESTNET_URL = "http://testnet.securesecrets.org:1317/";

export const CUSTOM_FEES = {
  upload: {
    amount: [{ amount: "2500000", denom: "uscrt" }],
    gas: "10000000",
  },
  init: {
    amount: [{ amount: "2500000", denom: "uscrt" }],
    gas: "10000000",
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
