#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod testnet_faucet {
    use ink::storage::Mapping;

    /// Error type for the faucet contract
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Transfer failed
        TransferFailed,
        /// Not enough time has passed since last funding
        CooldownNotExpired,
        /// Not authorized to call this function
        NotAuthorized,
        /// Arithmetic operation failed
        ArithmeticError,
        /// Token contract call failed
        TokenContractError,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum TokenType {
        LLD,
        LLM,
    }

    /// Minimal PSP22 token interface for cross-contract calls
    #[ink::trait_definition]
    pub trait PSP22 {
        #[ink(message)]
        fn transfer(&mut self, to: AccountId, value: Balance) -> Result<(), ()>;
        
        #[ink(message)]
        fn balance_of(&self, owner: AccountId) -> Balance;
    }

    #[ink(event)]
    pub struct RequestFundsEvent {
        #[ink(topic)]
        recipient: AccountId,
        #[ink(topic)]
        amount: Balance,
        #[ink(topic)]
        token_type: TokenType,
    }

    /// Testnet faucet that distributes LLD and LLM tokens
    #[ink(storage)]
    pub struct TestnetFaucet {
        /// Tracks last request timestamp for each account for LLD
        lld_fundings: Mapping<AccountId, u64>,
        /// Amount to fund each request (in LLD units)
        lld_funding_amount: Balance,
        /// Cooldown period in milliseconds (default: 1 day)
        lld_cooldown_period: u64,
        /// Tracks last request timestamp for each account for LLM
        llm_fundings: Mapping<AccountId, u64>,
        /// Amount to fund each request (in LLM units)
        llm_funding_amount: Balance,
        /// Cooldown period in milliseconds (default: 1 day)
        llm_cooldown_period: u64,
        /// LLM token contract address
        llm_token_contract: Option<AccountId>,
        /// Contract owner
        owner: AccountId,
    }

    impl TestnetFaucet {
        /// Creates a new testnet faucet
        #[ink(constructor)]
        pub fn new(
            lld_funding_amount: Balance,
            lld_cooldown_period: u64,
            llm_funding_amount: Balance,
            llm_cooldown_period: u64,
            llm_token_contract: Option<AccountId>,
        ) -> Self {
            Self {
                lld_fundings: Mapping::default(),
                lld_funding_amount,
                lld_cooldown_period,
                llm_fundings: Mapping::default(),
                llm_funding_amount,
                llm_cooldown_period,
                llm_token_contract,
                owner: Self::env().caller(),
            }
        }

        /// Creates a new testnet faucet with default values
        /// Funding amount: 1000 LLD/LLM
        /// Cooldown period: 1 day (86,400,000 milliseconds)
        /// LLM token contract: None (must be set later)
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(
                1000 * 1_000_000_000_000,
                86_400_000,
                1000 * 1_000_000_000_000,
                86_400_000,
                None,
            )
        }

        /// Set the LLM token contract address (only owner)
        #[ink(message)]
        pub fn set_llm_token_contract(&mut self, contract_address: AccountId) {
            assert!(self.env().caller() == self.owner, "Only owner can set token contract");
            self.llm_token_contract = Some(contract_address);
        }

        /// Request funds for a specific wallet
        /// Only contract owner can call this function
        /// Returns Ok if successful, Error otherwise
        #[ink(message)]
        pub fn fund_account(
            &mut self,
            recipient: AccountId,
            token_type: TokenType,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            // Only owner can call this function
            if caller != self.owner {
                return Err(Error::NotAuthorized);
            }

            let current_time = self.env().block_timestamp();

            // Check if recipient account is in cooldown period
            if let Some(last_request) = match token_type {
                TokenType::LLD => self.lld_fundings.get(recipient),
                TokenType::LLM => self.llm_fundings.get(recipient),
            } {
                let expiry_time = last_request
                    .checked_add(match token_type {
                        TokenType::LLD => self.lld_cooldown_period,
                        TokenType::LLM => self.llm_cooldown_period,
                    })
                    .ok_or(Error::ArithmeticError)?;

                if current_time < expiry_time {
                    return Err(Error::CooldownNotExpired);
                }
            }

            // Update last request timestamp for recipient
            match token_type {
                TokenType::LLD => {
                    self.lld_fundings.insert(recipient, &current_time);
                }
                TokenType::LLM => {
                    self.llm_fundings.insert(recipient, &current_time);
                }
            }

            // Transfer funds to recipient
            let amount = match token_type {
                TokenType::LLD => {
                    // Native currency transfer
                    if self.env().transfer(recipient, self.lld_funding_amount).is_err() {
                        return Err(Error::TransferFailed);
                    }
                    self.lld_funding_amount
                }
                TokenType::LLM => {
                    // Token contract call
                    if let Some(token_contract_address) = self.llm_token_contract {
                        let mut token_contract: ink::contract_ref!(PSP22) = 
                            token_contract_address.into();
                        
                        if token_contract.transfer(recipient, self.llm_funding_amount).is_err() {
                            return Err(Error::TokenContractError);
                        }
                    } else {
                        return Err(Error::TokenContractError);
                    }
                    self.llm_funding_amount
                }
            };

            self.env().emit_event(RequestFundsEvent {
                recipient,
                amount,
                token_type,
            });

            Ok(())
        }

        /// Check when a wallet can request funds again
        #[ink(message)]
        pub fn time_until_next_request(
            &self,
            account: AccountId,
            token_type: TokenType,
        ) -> Result<Option<u64>, Error> {
            let current_time = self.env().block_timestamp();

            if let Some(last_request) = match token_type {
                TokenType::LLD => self.lld_fundings.get(account),
                TokenType::LLM => self.llm_fundings.get(account),
            } {
                let next_valid_request = last_request
                    .checked_add(match token_type {
                        TokenType::LLD => self.lld_cooldown_period,
                        TokenType::LLM => self.llm_cooldown_period,
                    })
                    .ok_or(Error::ArithmeticError)?;

                if current_time < next_valid_request {
                    return Ok(Some(
                        next_valid_request
                            .checked_sub(current_time)
                            .ok_or(Error::ArithmeticError)?,
                    ));
                }
            }

            Ok(None) // Can request now
        }

        /// Allows owner to change the funding amount
        #[ink(message)]
        pub fn set_funding_amount(&mut self, new_amount: Balance, token_type: TokenType) {
            assert!(
                self.env().caller() == self.owner,
                "Only owner can change parameters"
            );
            match token_type {
                TokenType::LLD => {
                    self.lld_funding_amount = new_amount;
                }
                TokenType::LLM => {
                    self.llm_funding_amount = new_amount;
                }
            }
        }

        /// Allows owner to change the cooldown period
        #[ink(message)]
        pub fn set_cooldown_period(&mut self, new_period: u64, token_type: TokenType) {
            assert!(
                self.env().caller() == self.owner,
                "Only owner can change parameters"
            );
            match token_type {
                TokenType::LLD => {
                    self.lld_cooldown_period = new_period;
                }
                TokenType::LLM => {
                    self.llm_cooldown_period = new_period;
                }
            }
        }

        /// Allows owner to withdraw LLD from the contract
        #[ink(message)]
        pub fn withdraw_lld(&mut self, amount: Balance) -> Result<(), Error> {
            assert!(self.env().caller() == self.owner, "Only owner can withdraw");

            if self.env().transfer(self.owner, amount).is_err() {
                return Err(Error::TransferFailed);
            }

            Ok(())
        }

        /// Check LLM token balance of the faucet contract
        #[ink(message)]
        pub fn get_llm_balance(&self) -> Result<Balance, Error> {
            if let Some(token_contract_address) = self.llm_token_contract {
                let token_contract: ink::contract_ref!(PSP22) = 
                    token_contract_address.into();
                Ok(token_contract.balance_of(self.env().account_id()))
            } else {
                Err(Error::TokenContractError)
            }
        }

        /// Get contract's native balance
        #[ink(message)]
        pub fn get_native_balance(&self) -> Balance {
            self.env().balance()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::{test, DefaultEnvironment};

        // Helper function to advance block timestamp by specified milliseconds
        fn advance_timestamp(duration_ms: u64) {
            let current_timestamp = test::block_timestamp::<DefaultEnvironment>();
            test::set_block_timestamp::<DefaultEnvironment>(current_timestamp + duration_ms);
        }

        #[ink::test]
        fn default_works() {
            let faucet = TestnetFaucet::default();
            assert_eq!(faucet.lld_funding_amount, 1000 * 1_000_000_000_000);
            assert_eq!(faucet.lld_cooldown_period, 86_400_000);
            assert_eq!(faucet.llm_funding_amount, 1000 * 1_000_000_000_000);
            assert_eq!(faucet.llm_cooldown_period, 86_400_000);
            assert_eq!(faucet.llm_token_contract, None);
        }

        #[ink::test]
        fn fund_account_lld_works() {
            // Create a new faucet with 10 token funding
            let mut faucet = TestnetFaucet::new(10, 1000, 10, 1000, None);

            // Set contract balance and accounts
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_account_balance::<DefaultEnvironment>(accounts.alice, 100);

            // Set the caller to be the owner (contract creator)
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            // Test account to fund
            let recipient = accounts.bob;

            // Request LLD funds should succeed
            assert!(faucet.fund_account(recipient, TokenType::LLD).is_ok());

            // Second request for same recipient should fail due to cooldown
            assert_eq!(
                faucet.fund_account(recipient, TokenType::LLD),
                Err(Error::CooldownNotExpired)
            );

            // Advance time
            advance_timestamp(1001);

            // Now the request should succeed
            assert!(faucet.fund_account(recipient, TokenType::LLD).is_ok());

            // Test authorization
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert_eq!(
                faucet.fund_account(accounts.charlie, TokenType::LLD),
                Err(Error::NotAuthorized)
            );
        }

        #[ink::test]
        fn fund_account_llm_without_contract_fails() {
            let mut faucet = TestnetFaucet::default();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            // Should fail because no LLM token contract is set
            assert_eq!(
                faucet.fund_account(accounts.bob, TokenType::LLM),
                Err(Error::TokenContractError)
            );
        }

        #[ink::test]
        fn set_llm_token_contract_works() {
            let mut faucet = TestnetFaucet::default();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            // Set LLM token contract
            faucet.set_llm_token_contract(accounts.charlie);
            assert_eq!(faucet.llm_token_contract, Some(accounts.charlie));
        }

        #[ink::test]
        fn time_until_next_request_works() {
            // Create a new faucet
            let mut faucet = TestnetFaucet::new(10, 1000, 10, 1000, None);

            // Get accounts
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            // Initially should return None (can request)
            assert_eq!(
                faucet
                    .time_until_next_request(accounts.bob, TokenType::LLD)
                    .unwrap(),
                None
            );

            // Fund the account
            assert!(faucet.fund_account(accounts.bob, TokenType::LLD).is_ok());

            // Now should return Some time
            assert!(faucet
                .time_until_next_request(accounts.bob, TokenType::LLD)
                .unwrap()
                .is_some());

            // Advance time partially
            advance_timestamp(500);

            // Should still return Some time, but less
            let time_left = faucet
                .time_until_next_request(accounts.bob, TokenType::LLD)
                .unwrap()
                .unwrap();
            assert!(time_left > 0 && time_left <= 500);

            // Advance time fully
            advance_timestamp(1000);

            // Should return None again
            assert_eq!(
                faucet
                    .time_until_next_request(accounts.bob, TokenType::LLD)
                    .unwrap(),
                None
            );
        }
    }
}
