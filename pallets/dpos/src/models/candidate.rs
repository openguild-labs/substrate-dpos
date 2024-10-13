use frame::deps::frame_support::{
	sp_runtime::traits::{CheckedAdd, CheckedSub},
	traits::DefensiveSaturating,
};

use codec::{Decode, Encode, MaxEncodedLen};

use sp_runtime::traits::Zero;
use scale_info::TypeInfo;
use sp_core::RuntimeDebug;
use crate::{BalanceOf, Config};
use super::DispatchResultWithValue;

/// The `Candidate` struct represents a candidate in the DPoS system.
/// It includes the candidate's bond and the sum of delegated amounts.
#[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
#[scale_info(skip_type_params(T))]
pub struct Candidate<T: Config> {
	/// The bond amount staked by the candidate.
    pub bond: BalanceOf<T>,
	/// The total amount delegated to the candidate.
    pub sum_delegation: BalanceOf<T>,
}

impl<T: Config> Candidate<T> {
	/// Creates a new candidate with the given bond.
    ///
    /// # Arguments
    ///
    /// * `bond` - The bond amount staked by the candidate.
    ///
    /// # Returns
    ///
    /// A new `Candidate` instance.
    pub fn new(bond: BalanceOf<T>) -> Self {
        Self {
            bond,
            sum_delegation: Zero::zero(),
        }
    }

    /// Adds the given amount to the candidate's delegated sum.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount to add to the delegated sum.
    ///
    /// # Returns
    ///
    /// A `DispatchResultWithValue` containing the updated delegated sum.
    ///
    /// # Panics
    ///
    /// Panics if the addition results in an overflow.
    pub fn add_delegated_amount(
		&mut self,
		amount: BalanceOf<T>,
	) -> DispatchResultWithValue<BalanceOf<T>> {
		self.sum_delegation = self.sum_delegation.checked_add(&amount).expect("Overflow");
		Ok(self.sum_delegation)
	}

	/// Subtracts the given amount from the candidate's delegated sum.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount to subtract from the delegated sum.
    ///
    /// # Returns
    ///
    /// A `DispatchResultWithValue` containing the updated delegated sum.
    ///
    /// # Panics
    ///
    /// Panics if the subtraction results in an overflow.
	pub fn sub_delegated_amount(
		&mut self,
		amount: BalanceOf<T>,
	) -> DispatchResultWithValue<BalanceOf<T>> {
		self.sum_delegation = self.sum_delegation.checked_sub(&amount).expect("Overflow");
		Ok(self.sum_delegation)
	}

	/// Updates the candidate's bond to the given amount.
    ///
    /// # Arguments
    ///
    /// * `bond` - The new bond amount.
    pub fn update_bond(&mut self, bond: BalanceOf<T>) {
		self.bond = bond;
	}

	/// Returns the total amount staked by the candidate, including both the bond and the delegated sum.
    ///
    /// # Returns
    ///
    /// The total amount staked by the candidate.
	pub fn total(&self) -> BalanceOf<T> {
		self.sum_delegation.defensive_saturating_add(self.bond)
	}
}

/// A type alias for a set of candidates, represented as a vector of tuples containing the candidate's account ID and bond amount.
#[allow(type_alias_bounds)]
pub type CandidateSet<T: Config> = sp_std::vec::Vec<(T::AccountId, BalanceOf<T>)>;