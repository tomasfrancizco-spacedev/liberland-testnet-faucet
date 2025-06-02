# Liberland Testnet Faucet

A simple ink! smart contract that serves as a faucet registry for the Liberland testnet, tracking the distribution of LLD and LLM tokens to users.

## Overview

This faucet contract enables controlled tracking of testnet token distributions. Key features:

- Tracks distribution of two token types: LLD (Liberland Dollar) and LLM (Liberland Merit)
- Distributes 1000 LLD tokens per request (12 decimals)
- Distributes 10 LLM tokens per request (12 decimals)
- Implements a 24-hour cooldown period between requests for each token type per wallet
- Functions as a registry - actual token transfers are handled by a backend service
- Restricts recording functions to the contract owner only
- Includes administrative functions for parameter adjustments

## Contract Functions

### Recording Functions (Owner-Only)

- `record_lld_funding(receiver: AccountId)`: Records that LLD tokens were distributed to a receiver, starts 24-hour cooldown
- `record_llm_funding(receiver: AccountId)`: Records that LLM tokens were distributed to a receiver, starts 24-hour cooldown
- `record_funding(receiver: AccountId, token_type: TokenType)`: Combined function to record funding for either LLD or LLM

### Query Functions (Public)

- `can_fund_now(account: AccountId, token_type: TokenType)`: Checks if an account is eligible for immediate funding
- `get_last_funding(account: AccountId, token_type: TokenType)`: Returns the timestamp of the last funding
- `get_time_until_next_funding(account: AccountId, token_type: TokenType)`: Returns milliseconds until next funding is available
- `get_lld_amount()`: Returns current LLD funding amount (1000 LLD = 1,000,000,000,000,000 units)
- `get_llm_amount()`: Returns current LLM funding amount (10 LLM = 10,000,000,000,000 units)
- `get_funding_period()`: Returns the cooldown period in milliseconds (default: 86,400,000 ms = 24 hours)
- `get_owner()`: Returns the contract owner address

### Administrative Functions (Owner-Only)

- `change_owner(new_owner: AccountId)`: Transfers contract ownership
- `set_lld_amount(amount: Balance)`: Changes the LLD amount distributed per request
- `set_llm_amount(amount: Balance)`: Changes the LLM amount distributed per request
- `set_funding_period(period_ms: u64)`: Adjusts the cooldown period in milliseconds

## Token Types

The contract supports two token types defined in the `TokenType` enum:
- `TokenType::LLD` - Liberland Dollar
- `TokenType::LLM` - Liberland Merit

Each token type has independent cooldown tracking, meaning users can receive both LLD and LLM tokens, but must wait 24 hours between requests for each specific token type.

## Building the Contract

### Prerequisites

- Rust and Cargo installed
- ink! development environment set up
- Liberland blockchain environment

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
3. The contract initializes with default values:
   - LLD funding amount: 1,000,000,000,000,000 units (1000 LLD with 12 decimals)
   - LLM funding amount: 10,000,000,000,000 units (10 LLM with 12 decimals)
   - Cooldown period: 86,400,000 ms (24 hours)
   - Owner: deployer's account

## Usage

The contract is designed to work with a backend service that manages the actual token distribution. The workflow is:

1. Backend receives user requests for testnet tokens
2. Backend transfers the actual tokens to the user's address
3. Backend calls the appropriate recording function (`record_lld_funding` or `record_llm_funding`) to log the distribution
4. The contract enforces cooldown periods for future requests

## Events

The contract emits the following events:

- `LLDFundingRecorded`: Emitted when LLD funding is recorded
- `LLMFundingRecorded`: Emitted when LLM funding is recorded  
- `OwnerChanged`: Emitted when contract ownership changes

## Error Types

- `TooEarlyForFunding`: Account has already been funded within the 24-hour period
- `OnlyOwner`: Only the owner can call this function
- `InvalidAmount`: Invalid amount (cannot be zero)