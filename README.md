# Jito-BLS-NCN

## Paradigms

- AFCAP - As Few Crates As Possible. I have made sure we are using the very minimum amount of crates needed for the project. This is to minimize future maintenence and dependancy hell. No serde, no bytemuck, mostly just solana crates or crates that anza uses.

- Feature Flags - Sometimes external crates are needed, in this case, they should be feature gated and "off" by default.

- Heirarchy of packages, from least dependancies to most. More specifically we have:
- - `core` - Outlines all of the instructions and on-chain accounts
- - `program` - implements the on-chain instructions using everything from `core`
- - `sdk` - creates a nice wrapper around the instructions and accounts
- - `clients` - Allows you to interact with the program given a `Client`

- Reusable clients - Code used in the `integration_tests`, `clis` and `crankers` can all use the same code created in the `clients` package. And with the flexable `JitoClient` we can run integration tests with surfpool or locally.

- `JitoClient` - A wrapper around solana client-side execution that allows code to be run with `solana-program-test`, `surfpool` or a regular `rpc-client`.

- Everything is a Pod - All data fields for on-chain accounts and ix-data use Pods to keep everything 1 byte aligned. This allows us to stay away from external serialize/deserialize packages.

- Everything is handrolled - This continues the theme of AFCAP. All accounts and instructions, at the end of the day, are just bytes. In conjunction with our handrolled Pod implementation, we don't need anything fancy to know what we're doing. This takes up a bit more time on the dev side, but greatly simplifies the codebase. Transparent and fast.

- `JitoAccount` and `JitoIxData` as traits - Outlines what needs to be implemented for an account of instruction data such that we can have seperation of generic account checks and specific implementation of functions or modifiers for.

- Update or modification logic should live in the account's implementation, no direct manipulation of the account in the instruction. This keeps it all in one place and easier for auditability.

- Generic and reusable functions that can be used across all Jito Accounts, like `load_account`, `check_system_account`, `create_or_realloc`, `close_account`, ... outlined in `utils.rs`

- External packages to standardize to: `anyhow`, `tokio`, `log`, `clap`. Errors should use `anyhow` and `log` for logging.

- Check, Transact, Modify flow for every instruction.

- No more `realloc` functions, everything is `initialize` - The instruction should always call `create_or_realloc`. This improves readability/organization and allows for eaiser expansion in the future.

- On-chain code should be more liberal with its use of `msg!`. Unless we need performance, more info is better. Like `msg!("Hit consensus using {}/{} operators on message {}", num_operators, total_operators, message_hash)`

- A lot of this code should be moved to a `jito-core` repository.

## Testing

*Run with `solana-program-test`:*
```bash
./test.sh
```

*Run with surfpool:*

In a terminal:
```bash
surfpool start
```

In another terminal:
```bash
./test.sh surfpool
```
