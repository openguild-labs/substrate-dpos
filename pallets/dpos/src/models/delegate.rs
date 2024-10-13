use crate::{BalanceOf, Config};
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::RuntimeDebug;


/// The `Delegation` struct represents a delegation in the DPoS system.
/// It includes the amount of tokens delegated by a delegator to a candidate.
#[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
#[scale_info(skip_type_params(T))]
pub struct Delegation<T: Config> {
    /// The amount of tokens delegated.
    pub amount: BalanceOf<T>,
}

impl<T: Config> Delegation<T> {
    /// Creates a new delegation with the given amount.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount of tokens to delegate.
    ///
    /// # Returns
    ///
    /// A new `Delegation` instance.
    pub fn new(amount: BalanceOf<T>) -> Self {
        Self { amount }
    }
    
    /// Sets the amount of tokens delegated.
    ///
    /// # Arguments
    ///
    /// * `amount` - The new amount of tokens to delegate.
    pub fn set_amount(&mut self, amount: BalanceOf<T>) {
        self.amount = amount;
    }
}