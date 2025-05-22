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
    }

    /// Testnet faucet that distributes LLD tokens
    #[ink(storage)]
    pub struct TestnetFaucet {
        /// Tracks last request timestamp for each account
        fundings: Mapping<AccountId, u64>,
        /// Amount to fund each request (in LLD units)
        funding_amount: Balance,
        /// Cooldown period in milliseconds (default: 1 day)
        cooldown_period: u64,
        /// Contract owner
        owner: AccountId,
    }

    impl TestnetFaucet {
        /// Creates a new testnet faucet
        #[ink(constructor)]
        pub fn new(funding_amount: Balance, cooldown_period: u64) -> Self {
            Self {
                fundings: Mapping::default(),
                funding_amount,
                cooldown_period,
                owner: Self::env().caller(),
            }
        }

        /// Creates a new testnet faucet with default values
        /// Funding amount: 1000 LLD
        /// Cooldown period: 1 day (86,400,000 milliseconds)
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(1000 * 1_000_000_000_000, 86_400_000)
        }

        /// Request funds for a specific wallet
        /// Only contract owner can call this function
        /// Returns Ok if successful, Error otherwise
        #[ink(message)]
        pub fn request_funds(&mut self, recipient: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Only owner can call this function
            if caller != self.owner {
                return Err(Error::NotAuthorized);
            }
            
            let current_time = self.env().block_timestamp();
            
            // Check if recipient account is in cooldown period
            if let Some(last_request) = self.fundings.get(recipient) {
                let expiry_time = last_request.checked_add(self.cooldown_period)
                    .ok_or(Error::ArithmeticError)?;
                    
                if current_time < expiry_time {
                    return Err(Error::CooldownNotExpired);
                }
            }
            
            // Update last request timestamp for recipient
            self.fundings.insert(recipient, &current_time);
            
            // Transfer funds to recipient
            if self.env().transfer(recipient, self.funding_amount).is_err() {
                return Err(Error::TransferFailed);
            }
            
            Ok(())
        }
        
        /// Check when a wallet can request funds again
        #[ink(message)]
        pub fn time_until_next_request(&self, account: AccountId) -> Result<Option<u64>, Error> {
            let current_time = self.env().block_timestamp();
            
            if let Some(last_request) = self.fundings.get(account) {
                let next_valid_request = last_request.checked_add(self.cooldown_period)
                    .ok_or(Error::ArithmeticError)?;
                    
                if current_time < next_valid_request {
                    return Ok(Some(
                        next_valid_request.checked_sub(current_time)
                            .ok_or(Error::ArithmeticError)?
                    ));
                }
            }
            
            Ok(None) // Can request now
        }
        
        /// Allows owner to change the funding amount
        #[ink(message)]
        pub fn set_funding_amount(&mut self, new_amount: Balance) {
            assert!(self.env().caller() == self.owner, "Only owner can change parameters");
            self.funding_amount = new_amount;
        }
        
        /// Allows owner to change the cooldown period
        #[ink(message)]
        pub fn set_cooldown_period(&mut self, new_period: u64) {
            assert!(self.env().caller() == self.owner, "Only owner can change parameters");
            self.cooldown_period = new_period;
        }
        
        /// Allows owner to withdraw funds from the contract
        #[ink(message)]
        pub fn withdraw(&mut self, amount: Balance) -> Result<(), Error> {
            assert!(self.env().caller() == self.owner, "Only owner can withdraw");
            
            if self.env().transfer(self.owner, amount).is_err() {
                return Err(Error::TransferFailed);
            }
            
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::{test, DefaultEnvironment};

        #[ink::test]
        fn default_works() {
            let faucet = TestnetFaucet::default();
            assert_eq!(faucet.funding_amount, 1000 * 1_000_000_000_000);
            assert_eq!(faucet.cooldown_period, 86_400_000);
        }

        #[ink::test]
        fn request_funds_works() {
            // Create a new faucet with 10 token funding
            let mut faucet = TestnetFaucet::new(10, 1000);
            
            // Set contract balance and accounts
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_account_balance::<DefaultEnvironment>(accounts.alice, 100);
            
            // Set the caller to be the owner (contract creator)
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            
            // Test account to fund
            let recipient = accounts.bob;
            
            // Request funds should succeed
            assert!(faucet.request_funds(recipient).is_ok());
            
            // Second request for same recipient should fail due to cooldown
            assert_eq!(faucet.request_funds(recipient), Err(Error::CooldownNotExpired));
            
            // Advance time
            test::advance_time::<DefaultEnvironment>(1001);
            
            // Now the request should succeed
            assert!(faucet.request_funds(recipient).is_ok());
            
            // Test authorization
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert_eq!(faucet.request_funds(accounts.charlie), Err(Error::NotAuthorized));
        }
        
        #[ink::test]
        fn time_until_next_request_works() {
            // Create a new faucet
            let mut faucet = TestnetFaucet::new(10, 1000);
            
            // Get accounts
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            
            // Initially should return None (can request)
            assert_eq!(faucet.time_until_next_request(accounts.bob).unwrap(), None);
            
            // Fund the account
            assert!(faucet.request_funds(accounts.bob).is_ok());
            
            // Now should return Some time
            assert!(faucet.time_until_next_request(accounts.bob).unwrap().is_some());
            
            // Advance time partially
            test::advance_time::<DefaultEnvironment>(500);
            
            // Should still return Some time, but less
            let time_left = faucet.time_until_next_request(accounts.bob).unwrap().unwrap();
            assert!(time_left > 0 && time_left <= 500);
            
            // Advance time fully
            test::advance_time::<DefaultEnvironment>(1000);
            
            // Should return None again
            assert_eq!(faucet.time_until_next_request(accounts.bob).unwrap(), None);
        }
    }
}
