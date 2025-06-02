#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract(env = liberland_extension::LiberlandEnvironment)]
mod faucet {
    use ink::storage::Mapping;

    /// Custom errors for the faucet contract
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Account has already been funded within the 24-hour period
        TooEarlyForFunding,
        /// Only the owner can call this function
        OnlyOwner,
        /// Invalid amount
        InvalidAmount,
    }

    /// Events emitted by the faucet contract
    #[ink(event)]
    pub struct LLDFundingRecorded {
        #[ink(topic)]
        receiver: AccountId,
        amount: Balance,
        timestamp: Timestamp,
    }

    #[ink(event)]
    pub struct LLMFundingRecorded {
        #[ink(topic)]
        receiver: AccountId,
        amount: Balance,
        timestamp: Timestamp,
    }

    #[ink(event)]
    pub struct OwnerChanged {
        #[ink(topic)]
        old_owner: AccountId,
        #[ink(topic)]
        new_owner: AccountId,
    }

    /// Token type for funding
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum TokenType {
        LLD,
        LLM,
    }

    /// The faucet registry contract storage
    #[ink(storage)]
    pub struct Faucet {
        /// Owner of the faucet contract (backend EOA)
        owner: AccountId,
        /// Mapping from receiver address to last LLD funding timestamp
        lld_fundings: Mapping<AccountId, Timestamp>,
        /// Mapping from receiver address to last LLM funding timestamp
        llm_fundings: Mapping<AccountId, Timestamp>,
        /// Amount of LLD to fund per request (in smallest unit)
        lld_amount: Balance,
        /// Amount of LLM to fund per request (in smallest unit)
        llm_amount: Balance,
        /// Time period that must pass between fundings (24 hours in milliseconds)
        funding_period: u64,
    }

    impl Faucet {
        /// Constructor for the faucet registry contract
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                owner: caller,
                lld_fundings: Mapping::default(),
                llm_fundings: Mapping::default(),
                lld_amount: 1_000_000_000_000_000, // 1000 LLD (12 decimals)
                llm_amount: 10_000_000_000_000,    // 10 LLM (12 decimals)
                funding_period: 24 * 60 * 60 * 1000, // 24 hours in milliseconds
            }
        }

        /// Record LLD funding for an account (called by backend after sending tokens)
        /// Can only be called by the owner
        #[ink(message)]
        pub fn record_lld_funding(&mut self, receiver: AccountId) -> Result<(), Error> {
            // Check if caller is owner
            if self.env().caller() != self.owner {
                return Err(Error::OnlyOwner);
            }

            // Update the last funding timestamp
            let current_time = self.env().block_timestamp();
            self.lld_fundings.insert(receiver, &current_time);

            // Emit event
            self.env().emit_event(LLDFundingRecorded {
                receiver,
                amount: self.lld_amount,
                timestamp: current_time,
            });

            Ok(())
        }

        /// Record LLM funding for an account (called by backend after sending tokens)
        /// Can only be called by the owner
        #[ink(message)]
        pub fn record_llm_funding(&mut self, receiver: AccountId) -> Result<(), Error> {
            // Check if caller is owner
            if self.env().caller() != self.owner {
                return Err(Error::OnlyOwner);
            }

            // Update the last funding timestamp
            let current_time = self.env().block_timestamp();
            self.llm_fundings.insert(receiver, &current_time);

            // Emit event
            self.env().emit_event(LLMFundingRecorded {
                receiver,
                amount: self.llm_amount,
                timestamp: current_time,
            });

            Ok(())
        }

        /// Record funding for an account with specified token type
        /// Combined function for convenience
        #[ink(message)]
        pub fn record_funding(
            &mut self,
            receiver: AccountId,
            token_type: TokenType,
        ) -> Result<(), Error> {
            match token_type {
                TokenType::LLD => self.record_lld_funding(receiver),
                TokenType::LLM => self.record_llm_funding(receiver),
            }
        }

        /// Check if an account is eligible for funding
        fn check_funding_eligibility(
            &self,
            receiver: &AccountId,
            token_type: TokenType,
        ) -> Result<(), Error> {
            let current_time = self.env().block_timestamp();

            let last_funding = match token_type {
                TokenType::LLD => self.lld_fundings.get(receiver),
                TokenType::LLM => self.llm_fundings.get(receiver),
            };

            if let Some(last_funding) = last_funding {
                let next_funding_time = last_funding.saturating_add(self.funding_period);
                if current_time < next_funding_time {
                    return Err(Error::TooEarlyForFunding);
                }
            }

            Ok(())
        }

        /// Get the last funding timestamp for an account and token type
        #[ink(message)]
        pub fn get_last_funding(
            &self,
            account: AccountId,
            token_type: TokenType,
        ) -> Option<Timestamp> {
            match token_type {
                TokenType::LLD => self.lld_fundings.get(account),
                TokenType::LLM => self.llm_fundings.get(account),
            }
        }

        /// Get the time remaining until next funding is available for an account and token type
        #[ink(message)]
        pub fn get_time_until_next_funding(
            &self,
            account: AccountId,
            token_type: TokenType,
        ) -> u64 {
            let current_time = self.env().block_timestamp();

            let last_funding = match token_type {
                TokenType::LLD => self.lld_fundings.get(account),
                TokenType::LLM => self.llm_fundings.get(account),
            };

            if let Some(last_funding) = last_funding {
                let next_funding_time = last_funding.saturating_add(self.funding_period);
                if current_time < next_funding_time {
                    return next_funding_time.saturating_sub(current_time);
                }
            }

            0 // Can fund immediately
        }

        /// Check if an account can be funded right now for a specific token type
        #[ink(message)]
        pub fn can_fund_now(&self, account: AccountId, token_type: TokenType) -> bool {
            self.check_funding_eligibility(&account, token_type).is_ok()
        }

        /// Get the current owner
        #[ink(message)]
        pub fn get_owner(&self) -> AccountId {
            self.owner
        }

        /// Get the LLD funding amount
        #[ink(message)]
        pub fn get_lld_amount(&self) -> Balance {
            self.lld_amount
        }

        /// Get the LLM funding amount
        #[ink(message)]
        pub fn get_llm_amount(&self) -> Balance {
            self.llm_amount
        }

        /// Get the funding period (in milliseconds)
        #[ink(message)]
        pub fn get_funding_period(&self) -> u64 {
            self.funding_period
        }

        /// Change the owner (only current owner can call this)
        #[ink(message)]
        pub fn change_owner(&mut self, new_owner: AccountId) -> Result<(), Error> {
            if self.env().caller() != self.owner {
                return Err(Error::OnlyOwner);
            }

            let old_owner = self.owner;
            self.owner = new_owner;

            self.env().emit_event(OwnerChanged {
                old_owner,
                new_owner,
            });

            Ok(())
        }

        /// Update the LLD funding amount (only owner can call this)
        #[ink(message)]
        pub fn set_lld_amount(&mut self, amount: Balance) -> Result<(), Error> {
            if self.env().caller() != self.owner {
                return Err(Error::OnlyOwner);
            }

            if amount == 0 {
                return Err(Error::InvalidAmount);
            }

            self.lld_amount = amount;
            Ok(())
        }

        /// Update the LLM funding amount (only owner can call this)
        #[ink(message)]
        pub fn set_llm_amount(&mut self, amount: Balance) -> Result<(), Error> {
            if self.env().caller() != self.owner {
                return Err(Error::OnlyOwner);
            }

            if amount == 0 {
                return Err(Error::InvalidAmount);
            }

            self.llm_amount = amount;
            Ok(())
        }

        /// Update the funding period (only owner can call this)
        #[ink(message)]
        pub fn set_funding_period(&mut self, period_ms: u64) -> Result<(), Error> {
            if self.env().caller() != self.owner {
                return Err(Error::OnlyOwner);
            }

            if period_ms == 0 {
                return Err(Error::InvalidAmount);
            }

            self.funding_period = period_ms;
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn constructor_works() {
            let faucet = Faucet::new();
            assert_eq!(faucet.get_lld_amount(), 1_000_000_000_000_000); // 1000 LLD
            assert_eq!(faucet.get_llm_amount(), 10_000_000_000_000); // 10 LLM
            assert_eq!(faucet.get_funding_period(), 24 * 60 * 60 * 1000); // 24 hours
        }

        #[ink::test]
        fn owner_can_record_funding() {
            let mut faucet = Faucet::new();
            let receiver = AccountId::from([0x1; 32]);

            // Owner should be able to check eligibility
            assert!(faucet.can_fund_now(receiver, TokenType::LLD));

            // Owner should be able to record funding
            let result = faucet.record_lld_funding(receiver);
            assert!(result.is_ok());

            // Verify that the funding timestamp was recorded
            assert!(faucet.get_last_funding(receiver, TokenType::LLD).is_some());

            // Verify cooldown is now active
            assert!(!faucet.can_fund_now(receiver, TokenType::LLD));
        }

        #[ink::test]
        fn funding_cooldown_works() {
            let faucet = Faucet::new();
            let receiver = AccountId::from([0x1; 32]);

            // Initially should be able to fund both tokens
            assert!(faucet.can_fund_now(receiver, TokenType::LLD));
            assert!(faucet.can_fund_now(receiver, TokenType::LLM));
            assert_eq!(
                faucet.get_time_until_next_funding(receiver, TokenType::LLD),
                0
            );
            assert_eq!(
                faucet.get_time_until_next_funding(receiver, TokenType::LLM),
                0
            );
        }

        #[ink::test]
        fn only_owner_can_change_settings() {
            let mut faucet = Faucet::new();
            let new_lld_amount = 2_000_000_000_000_000; // 2000 LLD
            let new_llm_amount = 20_000_000_000_000; // 20 LLM

            // Owner should be able to change amounts
            assert!(faucet.set_lld_amount(new_lld_amount).is_ok());
            assert_eq!(faucet.get_lld_amount(), new_lld_amount);

            assert!(faucet.set_llm_amount(new_llm_amount).is_ok());
            assert_eq!(faucet.get_llm_amount(), new_llm_amount);
        }

        #[ink::test]
        fn non_owner_cannot_record_funding() {
            let mut faucet = Faucet::new();
            let receiver = AccountId::from([0x1; 32]);

            // Change to a different account (simulate non-owner calling)
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(AccountId::from([0x2; 32]));

            let result_lld = faucet.record_lld_funding(receiver);
            assert_eq!(result_lld, Err(Error::OnlyOwner));

            let result_llm = faucet.record_llm_funding(receiver);
            assert_eq!(result_llm, Err(Error::OnlyOwner));
        }

        #[ink::test]
        fn combined_record_funding_works() {
            let mut faucet = Faucet::new();
            let receiver = AccountId::from([0x1; 32]);

            // Test combined function for LLD
            assert!(faucet.can_fund_now(receiver, TokenType::LLD));
            let result = faucet.record_funding(receiver, TokenType::LLD);
            assert!(result.is_ok());
            assert!(!faucet.can_fund_now(receiver, TokenType::LLD));

            // Test combined function for LLM
            assert!(faucet.can_fund_now(receiver, TokenType::LLM));
            let result = faucet.record_funding(receiver, TokenType::LLM);
            assert!(result.is_ok());
            assert!(!faucet.can_fund_now(receiver, TokenType::LLM));
        }
    }
}
