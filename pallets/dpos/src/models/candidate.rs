use frame::deps::frame_support::{
	sp_runtime::traits::{CheckedAdd, CheckedSub},
	traits::DefensiveSaturating,
};

use sp_runtime::traits::Zero;

use crate::{BalanceOf, Config};
use super::DispatchResultWithValue;

#[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
#[scale_info(skip_type_params(T))]
pub struct Candidate<T: Config> {
    pub bond: BalanceOf<T>,
    pub sum_delegation: BalanceOf<T>,
}

impl<T: Config> Candidate<T> {
    pub fn new(bond: BalanceOf<T>) -> Self {
        Self {
            bond,
            sum_delegation: Zero::zero(),
        }
    }

    pub fn add_delegated_amount(
		&mut self,
		amount: BalanceOf<T>,
	) -> DispatchResultWithValue<BalanceOf<T>> {
		self.sum_delegation = self.sum_delegation.checked_add(&amount).expect("Overflow");
		Ok(self.sum_delegation)
	}

    pub fn update_bond(&mut self, bond: BalanceOf<T>) {
		self.bond = bond;
	}

	pub fn total(&self) -> BalanceOf<T> {
		self.sum_delegation.defensive_saturating_add(self.bond)
	}
}

#[allow(type_alias_bounds)]
pub type CandidateSet<T: Config> = sp_std::vec::Vec<(T::AccountId, BalanceOf<T>)>;