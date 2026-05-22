# kamichain-cli

The `kami` binary. Talks to a running `kamichain-node` over the RPC TCP connection.

## Commands

```
kami wallet new                          # generate a keypair, print address, save keyfile
kami wallet address --keyfile <path>     # print address from a saved keyfile
kami wallet balance <address>            # query balance from the node

kami tx send --keyfile <path> --to <addr> --amount <n>   # sign and submit a transfer
kami tx get <id>                         # look up a transaction by ID

kami chain info                          # height, latest hash, difficulty
kami chain block <index>                 # full block details
kami chain validate                      # ask the node to validate its chain

kami mine --address <addr>               # instruct the node to mine one block

kami node start --bind <addr> --difficulty <n> [--peer <addr>]...   # start a node
```

## What to build

- `src/commands/wallet.rs` — implement `WalletCmd` variants
- `src/commands/tx.rs` — implement `TxCmd` variants
- `src/commands/chain.rs` — implement `ChainCmd` variants
- `src/main.rs` — parse args with `clap`, dispatch to command handlers, connect to node via TCP

All commands except `wallet new` and `node start` require a running node at `--node` (default `127.0.0.1:8332`).
