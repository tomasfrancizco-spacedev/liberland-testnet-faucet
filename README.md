# Liberland Testnet Faucet

A simple ink! smart contract that serves as a faucet for the Liberland testnet, distributing LLD tokens to users.

## Overview

This faucet contract enables controlled distribution of testnet LLD tokens. Key features:

- Distributes 1000 LLD tokens per request
- Implements a 24-hour cooldown period between requests for the same wallet
- Restricts faucet distribution to the contract owner only
- Includes administrative functions for parameter adjustments

## Contract Functions

### User Functions

- `request_funds(recipient: AccountId)`: Sends 1000 LLD to the specified recipient (only callable by owner)
- `time_until_next_request(account: AccountId)`: Checks when an account can request funds again

### Admin Functions

- `set_funding_amount(new_amount: Balance)`: Changes the amount distributed per request
- `set_cooldown_period(new_period: u64)`: Adjusts the cooldown period (in milliseconds)
- `withdraw(amount: Balance)`: Allows the owner to withdraw funds from the contract

## Building the Contract

### Prerequisites

- Rust and Cargo installed
- ink! development environment set up

### Build Instructions

1. Clone the repository:
   ```
   git clone [repository-url]
   cd testnet_faucet
   ```

2. Build the contract:
   ```
   cargo +nightly contract build
   ```

3. Run tests:
   ```
   cargo test
   ```

## Deployment

To deploy the contract to the Liberland testnet:

1. Build the contract as described above
2. Use the [Contracts UI](https://contracts-ui.substrate.io/) or polkadot.js to deploy the contract
3. During deployment, you can specify:
   - Initial funding amount (default: 1000 LLD)
   - Cooldown period in milliseconds (default: 86,400,000 ms = 24 hours)

## Usage

The contract is designed to be called by a backend service that manages the faucet distribution. The backend will:

1. Receive user requests for testnet tokens
2. Call the `request_funds` function with the user's address
3. The contract will enforce the cooldown period and distribute tokens