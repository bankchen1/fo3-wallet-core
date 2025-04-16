# FO3 Wallet Core

A Rust-driven multi-chain wallet and DeFi SDK supporting account management, mnemonic generation, asset synchronization, transaction broadcasting, and DEX contract interactions. This project serves as the core on-chain engine module for the FO3 digital wallet system, designed to be used by multiple platforms (App, Web, Admin).

## Project Structure

The project is organized into two main crates in a single repository:

1. `fo3-wallet`: Core library (lib) containing:
   - Mnemonic and private key management
   - Multi-chain key derivation
   - On-chain interactions
   - Transaction signing
   - DeFi protocol integrations

2. `fo3-wallet-api`: Axum-based REST API service (bin) that:
   - Exposes wallet-core functionality via HTTP endpoints
   - Provides a clean interface for client applications

## Supported Blockchains

- Ethereum and EVM-compatible chains
- Solana
- Bitcoin

## Features

- **Account Management**: Create, import, and manage wallets with BIP39 mnemonics
- **Multi-chain Support**: Derive addresses and keys for multiple blockchains
- **Transaction Handling**: Create, sign, and broadcast transactions
- **DeFi Integrations**: Interact with swaps, lending protocols, and staking platforms
- **Asset Management**: Track balances and transactions across chains

## Getting Started

### Prerequisites

- Rust 1.70+ and Cargo
- Development libraries for cryptographic functions

### Building the Project

```bash
# Clone the repository
git clone https://github.com/fo3/fo3-wallet-core.git
cd fo3-wallet-core

# Build the project
cargo build --release

# Run the API server
cargo run -p fo3-wallet-api
```

### Running Tests

```bash
cargo test
```

## API Documentation

The wallet-api exposes the following endpoints:

### Wallet Management

- `GET /wallets`: List all wallets
- `POST /wallets`: Create a new wallet
- `POST /wallets/import`: Import a wallet from mnemonic
- `GET /wallets/:id`: Get wallet details
- `PUT /wallets/:id`: Update wallet
- `DELETE /wallets/:id`: Delete wallet
- `GET /wallets/:id/addresses`: Get addresses for a wallet
- `POST /wallets/:id/addresses`: Derive a new address

### Transactions

- `GET /transactions`: List transactions
- `POST /transactions`: Create a new transaction
- `GET /transactions/:id`: Get transaction details
- `POST /transactions/:id/sign`: Sign a transaction
- `POST /transactions/:id/broadcast`: Broadcast a transaction

### DeFi

- `GET /defi/tokens/:address/balance`: Get token balance
- `GET /defi/swap/routes`: Get swap routes
- `POST /defi/swap/execute`: Execute a swap
- `GET /defi/lending/markets`: Get lending markets
- `GET /defi/lending/positions/:address`: Get lending positions
- `GET /defi/staking/pools`: Get staking pools
- `GET /defi/staking/positions/:address`: Get staking positions

## Future Enhancements

- WebAssembly (WASM) support for browser integration
- Additional blockchain support
- Enhanced security features
- Hardware wallet integration
- NFT support

## License

MIT License
