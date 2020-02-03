
# data-space

## Introduction
Data Space is  a bussiness platform for individual users and enterprises to make data transactions.  Now it is a simple implementation: 
On this platform, companies can publish their Order Forms to declare what data they want and how much money they can pay. For example, food manufacturers want to know what people eat last month so that they can make more popular food for sale this month.  
On the Data Space, all data are just belong to Users themselves and Companies cannot own their data if they don't allow.
The Data Space does not belong to anyone, it just belongs to the whole Internet.    

##Process
Alice: Company  
Bob: Person  
1. Alice publish an Order Form onto the chain, it contains what data it want to obtain 
and how much money can pay from people.  
2. Bob see the Order Form on the Data Space, he can choose to upload his data on chain.
3. Alice pays money for data on chain, the chain will lock Bob's Account temporarily.
4. Alice confirms the data is legal, (just not empty and not same as Order Form) the chain will unlock Bob's Account.
5. When Alice find the data is illegal, it can tip off Bob on chain, then the chain will validate data. 
If data is really illegal, the chain will keep locking Bob's Account. Otherwise Alice's Accout will be locked for punishment. 

# Building

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Install required tools:

```bash
./scripts/init.sh
```

Build the WebAssembly binary:

```bash
./scripts/build.sh
```

Build all native code:

```bash
cargo build
```

# Run

You can start a development chain with:

```bash
cargo run -- --dev
```

Detailed logs may be shown by running the node with the following environment variables set: `RUST_LOG=debug RUST_BACKTRACE=1 cargo run -- --dev`.

If you want to see the multi-node consensus algorithm in action locally, then you can create a local testnet with two validator nodes for Alice and Bob, who are the initial authorities of the genesis chain that have been endowed with testnet units. Give each node a name and expose them so they are listed on the Polkadot [telemetry site](https://telemetry.polkadot.io/#/Local%20Testnet). You'll need two terminal windows open.

We'll start Alice's substrate node first on default TCP port 30333 with her chain database stored locally at `/tmp/alice`. The bootnode ID of her node is `QmQZ8TjTqeDj3ciwr93EJ95hxfDsb9pEYDizUAbWpigtQN`, which is generated from the `--node-key` value that we specify below:

```bash
cargo run -- \
  --base-path /tmp/alice \
  --chain=local \
  --alice \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

In the second terminal, we'll start Bob's substrate node on a different TCP port of 30334, and with his chain database stored locally at `/tmp/bob`. We'll specify a value for the `--bootnodes` option that will connect his node to Alice's bootnode ID on TCP port 30333:

```bash
cargo run -- \
  --base-path /tmp/bob \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/QmQZ8TjTqeDj3ciwr93EJ95hxfDsb9pEYDizUAbWpigtQN \
  --chain=local \
  --bob \
  --port 30334 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

Additional CLI usage options are available and may be shown by running `cargo run -- --help`.
