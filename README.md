# 🚀 SCRT SIBYL CONTRACT

![scrt sibyl image](./images/logo_horizontal.png)

This is a secret contract in Rust to run in
[Secret Network](https://github.com/enigmampc/SecretNetwork).
To understand the framework better, please read the overview in the
[cosmwasm repo](https://github.com/CosmWasm/cosmwasm/blob/master/README.md),
and dig into the [cosmwasm docs](https://www.cosmwasm.com).

## At a Glance

SCRTSibyl is an oracle for credit scoring developed for the Secret Network. The oracle returns a numerical, private, and encrypted credit score affirming users' credibility and trustworthiness within the Secret Network ecosystem. The DApp was designed with one specific use case in mind: P2P micro-lending, which is facilitating lending and borrowing of microloans ranging between $1-25K USD. Running a credit score check on a user you are considering lending money to or borrowing money from, will inform you whether and how much a user can pay back upon loan issuance.

More info on the algo [here](https://github.com/BalloonBox-Inc/SCRTSibyl-Oracle).

---

## Execute Locally

First install a recent version of rust and cargo via [rustup](https://rustup.rs/).

Then install [cargo-generate](https://github.com/ashleygwilliams/cargo-generate).

```sh
cargo install cargo-generate --features vendored-openssl
```

Clone the repo:

```sh
git clone https://github.com/BalloonBox-Inc/SCRTSibyl-Contract.git
```

For testing:

```sh
cargo test
```

Note: add args `-- --nocapture` to debug tests

To build a wasm file:L

```sh
cargo wasm
```

You'll find the built file in /target/wasm32-unknown-unknown/release/name_of_your_contract.wasm

To build an optimized wasm file (without using Docker) run:

`make _build-mainnet`

then to copy it and zip it into the source folder:

`make compress-wasm`

you can check out the makefile file for the detailed scripts.
