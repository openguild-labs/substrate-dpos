use sp_std::collections::btree_map::BTreeMap;
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::RuntimeDebug;
use crate::{BalanceOf, Config};

/// The `Epoch` struct represents an epoch in the DPoS system.
/// It includes the validators and delegations for the epoch.
#[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
#[scale_info(skip_type_params(T))]
pub struct Epoch<T: Config> {
    /// A map of validators and their staked amounts.
	pub validators: BTreeMap<T::AccountId, BalanceOf<T>>,
 	/// A map of delegations, represented as tuples of delegator and candidate account IDs, and their delegated amounts.
	pub delegations: BTreeMap<(T::AccountId, T::AccountId), BalanceOf<T>>,
}

impl<T: Config> Epoch<T> {
    /// Creates a new epoch with default values.
    ///
    /// # Returns
    ///
    /// A new `Epoch` instance with empty validators and delegations.
	pub fn default() -> Self {
		Self { validators: BTreeMap::default(), delegations: BTreeMap::default() }
	}

    /// Adds a delegator to the epoch.
    ///
    /// # Arguments
    ///
    /// * `delegator` - The account ID of the delegator.
    /// * `candidate` - The account ID of the candidate.
    /// * `amount` - The amount of tokens delegated.
	pub fn add_delegator(
		self: &mut Self,
		delegator: T::AccountId,
		candidate: T::AccountId,
		amount: BalanceOf<T>,
	) {
		self.delegations.insert((delegator, candidate), amount);
	}

    /// Adds a validator to the epoch.
    ///
    /// # Arguments
    ///
    /// * `candidate` - The account ID of the candidate.
    /// * `amount` - The amount of tokens staked by the validator.
	pub fn add_validator(self: &mut Self, candidate: T::AccountId, amount: BalanceOf<T>) {
		self.validators.insert(candidate, amount);
	}
}