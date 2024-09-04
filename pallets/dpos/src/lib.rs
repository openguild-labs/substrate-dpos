#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod models;

// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/polkadot_sdk/frame_runtime/index.html
// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html
// https://paritytech.github.io/polkadot-sdk/master/frame_support/attr.pallet.html#dev-mode-palletdev_mode
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use crate::models::*;
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::{*, ValueQuery},
		traits::{
			fungible::{self, Mutate, MutateHold}, 
			FindAuthor,
		},
		sp_runtime::traits::{CheckedAdd, CheckedSub, Zero},
		sp_runtime::{traits::One, BoundedVec, Percent, Saturating},
		Twox64Concat,
	};
	use sp_std::prelude::*;
	use frame_system::pallet_prelude::{OriginFor, *};
	use sp_std:: {
		cmp::Reverse,
		vec::Vec,
		collections::{btree_map::BTreeMap, btree_set::BTreeSet}
	};

	pub trait ReportNewValidatorSet<AccountId> {
		fn report_new_validator_set(_new_set: Vec<AccountId>) {}
	}

	pub type BalanceOf<T> = <<T as Config>::NativeBalance as fungible::Inspect<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/reference_docs/frame_runtime_types/index.html
		type RuntimeEvent: From<Event<Self>>
		+ IsType<<Self as frame::deps::frame_system::Config>::RuntimeEvent>;

		/// Type to access the Balances Pallet.
		type NativeBalance: fungible::Inspect<Self::AccountId>
		+ fungible::Mutate<Self::AccountId>
		+ fungible::hold::Inspect<Self::AccountId>
		+ fungible::hold::Mutate<Self::AccountId>
		+ fungible::hold::Mutate<Self::AccountId, Reason = Self::RuntimeHoldReason>
		+ fungible::freeze::Inspect<Self::AccountId>
		+ fungible::freeze::Mutate<Self::AccountId>;

		/// The maximum number of authorities that the pallet can hold.
		type MaxValidators: Get<u32>;

		type MinValidators: Get<u32>;

		/// The maximum number of authorities that the pallet can hold.
		#[pallet::constant]
		type MaxCandidates: Get<u32>;

		/// The maximum number of delegators that the candidate can have
		/// If the number of delegators reaches the maximum, delegator with the lowest amount
		/// will be replaced by the new delegator if the new delegation is higher
		#[pallet::constant]
		type MaxCandidateDelegators: Get<u32>;

		#[pallet::constant]
		type MinDelegateAmount: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MinCandidateBond: Get<BalanceOf<Self>>;
		
		#[pallet::constant]
		type EpochDuration: Get<BlockNumberFor<Self>>;

		/// Find the author of a block. A fake provide for this type is provided in the runtime. You
		/// can use a similar mechanism in your tests.
		type FindAuthor: FindAuthor<Self::AccountId>;

		/// Report the new validators to the runtime. This is done through a custom trait defined in
		/// this pallet.
		type ReportNewValidatorSet: ReportNewValidatorSet<Self::AccountId>;
	}

	/// The pallet's storage items.
	/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#storage
	/// https://paritytech.github.io/polkadot-sdk/master/frame_support/pallet_macros/attr.storage.html
	#[pallet::storage]
	pub type CandidatePool<T: Config> = CountedStorageMap<_, Twox64Concat, T::AccountId, Candidate<T>, OptionQuery>;
	#[pallet::storage]
	pub type DelegateCountMap<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, u32, ValueQuery>;
	#[pallet::storage]
	pub type DelegationInfos<T: Config> = StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, T::AccountId, Delegation<T>, OptionQuery>;
	#[pallet::storage]
	pub type CandidateDelegators<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<T::AccountId, <T as Config>::MaxCandidateDelegators>, ValueQuery>;
	#[pallet::storage]
	pub type EpochIndex<T: Config> = StorageValue<_, u32, ValueQuery>;
	#[allow(type_alias_bounds)]
	pub type TopCandidateVec<T: Config> = sp_std::vec::Vec<(T::AccountId, BalanceOf<T>, BalanceOf<T>)>;
	#[pallet::storage]
	#[pallet::getter(fn current_validators)]
	pub type CurrentValidators<T: Config> = StorageValue<_, BoundedVec<(T::AccountId, BalanceOf<T>, BalanceOf<T>), <T as Config>::MaxValidators>, ValueQuery>;
	#[pallet::storage]
	#[pallet::unbounded]
	#[pallet::getter(fn last_epoch_snapshot)]
	pub type LastEpochSnapshot<T: Config> = StorageValue<_, Epoch<T>, OptionQuery>;
	#[pallet::storage]
	pub type Rewards<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	// genesis config
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub genesis_candidates: CandidateSet<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			assert!(
				T::MaxValidators::get() >= One::one(),
				"Need at least one validator for the network to function"
			);

			// Populates the provided genesis candidates with bond in storage.
			// Ensures that there are no duplicate candidates in the `genesis_candidates`.
			let mut visited: BTreeSet<T::AccountId> = BTreeSet::default();
			for (candidateId, bond) in self.genesis_candidates.iter() {
				assert!(visited.insert(candidateId.clone()), "Candidate registration duplicates");
				ensure!(
					CandidatePool::<T>::count().saturating_add(1) <= T::MaxCandidates::get(),
					Error::<T>::TooManyValidators
				);
				T::NativeBalance::hold(&HoldReason::CandidateBondReserved.into(), &candidateId, bond)?;
				let candidate = Candidate::new(bond);
				CandidatePool::<T>::insert(&candidateId, candidate);
				Pallet::<T>::deposit_event(Event::CandidateRegistered { candidate_id: candidateId, initial_bond: bond});
			}

			// Update the validator set using the data stored in the candidate pool
			let validator_set = Pallet::<T>::select_validator_set().to_vec();
			CurrentValidators::<T>::put(
				BoundedVec::try_from(validator_set.clone())
					.expect("Exceed limit number of the validators in the active set"),
			);
			// Capture the snapshot of the last epoch
			LastEpochSnapshot::<T>::set(Some(Pallet::<T>::capture_epoch_snapshot(
				&active_validator_set,
			)));

			let new_set = CurrentValidators::<T>::get()
				.iter()
				.map(|(validator, _, _)| validator.clone())
				.collect::<Vec<T::AccountId>>();

			Pallet::<T>::report_new_validators(new_set);
		}
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { genesis_candidates: vec![] }
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			Self::execute_rewards();
			let epoch_indx = n % T::EpochDuration::get();
			if epoch_indx == BlockNumberFor::<T>::zero() {
				let validator_set = Self::select_validator_set();

				CurrentValidators::<T>::put(
					BoundedVec::try_from(validator_set.to_vec())
						.expect("Exceed limit number of the validators in the active set"),
				);
				// In new epoch, we want to set the CurrentEpochSnapshot to the current dataset
				LastEpochSnapshot::<T>::set(Some(Pallet::<T>::capture_epoch_snapshot(
					&validator_set,
				)));

				let new_set = CurrentValidators::<T>::get()
					.iter()
					.map(|(active_validator, _, _)| active_validator.clone())
					.collect::<Vec<T::AccountId>>();

				Pallet::<T>::report_new_validators(new_set);
				Self::move_to_next_epoch(validator_set);
			}
			// We return a default weight because we do not expect you to do weights for your
			// project... Except for extra credit...
			return Weight::default();
		}

	}
	/// Pallets use events to inform users when important changes are made.
	/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// We usually use passive tense for events.
		SomethingStored { something: u32, who: T::AccountId },
		/// Event emitted when there is a new candidate registered
		CandidateRegistered { candidate_id: T::AccountId, initial_bond: BalanceOf<T> },
		CandidateRegistrationRemoved { candidate_id: T::AccountId },
		CandidateDelegated {
			candidate_id: T::AccountId,
			delegated_by: T::AccountId,
			amount: BalanceOf<T>,
			total_delegated_amount: BalanceOf<T>,
		},
		CandidateUndelegated {
			candidate_id: T::AccountId,
			delegator: T::AccountId,
			amount: BalanceOf<T>,
			left_delegated_amount: BalanceOf<T>,
		},
		NextEpochMoved {
			last_epoch: u32,
			next_epoch: u32,
			at_block: BlockNumberFor<T>,
			total_candidates: u64,
			total_validators: u64,
		},
		RewardClaimed { claimer: T::AccountId, total_reward: BalanceOf<T> },
	}

	/// Errors inform users that something went wrong.
	/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error
	#[pallet::error]
	pub enum Error<T> {
		TooManyValidators,
		InvalidZeroAmount,
		TooManyCandidateDelegations,
		TooManyDelegatorsInPool,
		CandidateAlreadyExist,
		CandidateDoesNotExist,
		DelegationDoesNotExist,
		BelowMinimumDelegateAmount,
		NoClaimableRewardFound,
	}

	/// A reason for the pallet dpos placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		/// Hold the candidate balance to reserve it for registration to the candidate pool.
		#[codec(index = 0)]
		CandidateBondReserved,
		/// Hold the amount delegated to the candidate
		#[codec(index = 1)]
		DelegateAmountReserved,
	}

	/// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	/// These functions materialize as "extrinsics", which are often compared to transactions.
	/// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#dispatchables
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example of directly updating the authorities into [`Config::ReportNewValidatorSet`].
		pub fn force_report_new_validators(
			origin: OriginFor<T>,
			new_set: Vec<T::AccountId>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(
				(new_set.len() as u32) < T::MaxValidators::get(),
				Error::<T>::TooManyValidators
			);
			Self::report_new_validators(new_set);
			Ok(())
		}

		pub fn register_as_candidate(
			origin: OriginFor<T>,
			initial_bond: BalanceOf<T>,
		) -> DispatchResult {
			ensure!(initial_bond > Zero::zero(), Error::<T>::InvalidZeroAmount);
			ensure!(initial_bond >= T::MinCandidateBond::get(), Error::<T>::BelowMinimumCandidateBond);

			let who = ensure_signed(origin)?;
			ensure!(!Self::is_candidate(&who), Error::<T>::CandidateAlreadyExist);
			ensure!(
				CandidatePool::<T>::count().saturating_add(1) <= T::MaxCandidates::get(),
				Error::<T>::TooManyValidators
			);

			T::NativeBalance::hold(&HoldReason::CandidateBondReserved.into(), &who, initial_bond)?;

			let candidate = Candidate::new(initial_bond);
			CandidatePool::<T>::insert(&who, candidate);
			Self::deposit_event(Event::CandidateRegistered { candidate_id: who, initial_bond });
			Ok(())
		}

		pub fn delegate(
			origin: OriginFor<T>,
			candidate: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure!(amount > Zero::zero(), Error::<T>::InvalidZeroAmount);
			let delegator = ensure_signed(origin)?;
			match DelegationInfos::<T>::try_get(&delegator, &candidate) {
				Ok(mut delegation_info) => {
					let new_delegated_amount =
						delegation_info.amount.checked_add(&amount).expect("Overflow");
					Self::check_delegated_amount(new_delegated_amount)?;
					delegation_info.set_amount(new_delegated_amount);
					DelegationInfos::<T>::set(&delegator, &candidate, Some(delegation_info));
				},
				Err(_) => {
					Self::check_delegated_amount(amount)?;
					let delegate_count = DelegateCountMap::<T>::get(&delegator);
					let new_delegate_count = delegate_count.saturating_add(1);
					ensure!(
						new_delegate_count <= T::MaxDelegateCount::get(),
						Error::<T>::TooManyCandidateDelegations
					);
					DelegateCountMap::<T>::set(&delegator, new_delegate_count);
					Self::add_candidate_delegator(&candidate, &delegator)?;
					let new_delegation_info = Delegation::new(amount);
					DelegationInfos::<T>::insert(&delegator, &candidate, new_delegation_info);
				},
			};

			T::NativeBalance::hold(&HoldReason::DelegateAmountReserved.into(), &delegator, amount)?;

			let total_delegated_amount = Self::increase_candidate_delegations(&candidate, &amount)?;

			Self::deposit_event(Event::CandidateDelegated {
				candidate_id: candidate,
				delegated_by: delegator,
				amount,
				total_delegated_amount,
			});
			Ok(())
		}

		pub fn unregister_as_candidate(origin: OriginFor<T>, candidate: T::AccountId) -> DispatchResult {
			T::ForceOrigin::ensure_origin(origin)?;
			ensure!(Self::is_candidate(&candidate), Error::<T>::CandidateDoesNotExist);
			let candidate_delegators = CandidateDelegators::<T>::get(&candidate);

			// Processing all the delegators of the candidate
			for delegator in candidate_delegators.into_inner() {
				let delegation_info = DelegationInfos::<T>::try_get(&delegator, &candidate)
					.map_err(|_| Error::<T>::DelegationDoesNotExist)?;

				// Trying to release all the hold amount of the delegators
				Self::release_delegated_amount(&delegator, &delegation_info.amount)?;

				// Removing any information related to the delegation between (candidate, delegator)
				Self::remove_candidate_delegation_data(&delegator, &candidate)?;
			}
			CandidateDelegators::<T>::remove(&who);

			// Releasing the hold bonds of the candidate
			let candidate_detail = Self::get_candidate(&candidate)?;
			Self::release_candidate_bonds(&candidate, candidate_detail.bond)?;
			let rewards = Rewards::<T>::get(&candidate);
			if rewards > Zero::zero() {
				let _ = T::NativeBalance::mint_into(&candidate, rewards);
				Rewards::<T>::remove(&candidate);
				Self::deposit_event(Event::RewardClaimed { claimer: candidate, total_reward: rewards });
			}
			// Removing any information related the registration of the candidate in the pool
			CandidatePool::<T>::remove(&candidate);

			Self::deposit_event(Event::CandidateRegistrationRemoved { candidate_id: who });

			Ok(())
		}

		pub fn undelegate(
			origin: OriginFor<T>,
			delegator: T::AccountId,
			candidate: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::ForceOrigin::ensure_origin(origin)?;
			ensure!(
				CandidatePool::<T>::contains_key(&candidate),
				Error::<T>::CandidateDoesNotExist
			);
			ensure!(amount > Zero::zero(), Error::<T>::InvalidZeroAmount);

			let mut delegation_info = Self::get_delegation(&delegator, &candidate)?;
			let new_delegated_amount = delegation_info
				.amount
				.checked_sub(&amount)
				.ok_or(Error::<T>::InvalidMinimumDelegateAmount)?;

			if new_delegated_amount.is_zero() {
				// If the delegated amount is removed completely, we want to remove
				// related information to the delegation betwene (delegator, candidate)
				Self::remove_candidate_delegation_data(&delegator, &candidate)?;
			} else {
				// Remove the delegated amoutn partially but makes sure it is still above
				// the minimum delegated amount
				Self::check_delegated_amount(new_delegated_amount)?;

				delegation_info.set_amount(new_delegated_amount);
				DelegationInfos::<T>::set(&delegator, &candidate, Some(delegation_info));
			}

			// Releasing the hold amount for the delegation betwene (delegator, candidate)
			Self::release_delegated_amount(&delegator, &amount)?;

			// Reduce the candidate total_delegation by the undelegated amount
			Self::decrease_candidate_delegations(&candidate, &amount)?;

			Self::deposit_event(Event::CandidateUndelegated {
				candidate_id: candidate,
				delegator,
				amount,
				left_delegated_amount: new_delegated_amount,
			});
			Ok(())
		}

		pub fn claim_reward(origin: OriginFor<T>) -> DispatchResult {
			let claimer = ensure_signed(origin)?;

			let rewards = Rewards::<T>::try_get(&claimer)
				.map_err(|_| Error::<T>::NoClaimableRewardFound)?;
			ensure!(rewards > Zero::zero(), Error::<T>::NoClaimableRewardFound);
			let _ = T::NativeBalance::mint_into(&claimer, rewards);
			Rewards::<T>::remove(&claimer);

			Self::deposit_event(Event::RewardClaimed { claimer, total_reward: rewards });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		// A function to get you an account id for the current block author.
		pub fn find_author() -> Option<T::AccountId> {
			// If you want to see a realistic example of the `FindAuthor` interface, see
			// `pallet-authorship`.
			T::FindAuthor::find_author::<'_, Vec<_>>(Default::default())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_delegation(
			delegator: &T::AccountId,
			candidate: &T::AccountId,
		) -> DispatchResultWithValue<Delegation<T>> {
			Ok(DelegationInfos::<T>::try_get(&delegator, &candidate)
				.map_err(|_| Error::<T>::DelegationDoesNotExist)?)
		}

		pub fn get_candidate(
			candidate: &T::AccountId,
		) -> DispatchResultWithValue<CandidateDetail<T>> {
			Ok(CandidatePool::<T>::try_get(&candidate)
				.map_err(|_| Error::<T>::CandidateDoesNotExist)?)
		}

		pub fn is_candidate(validator: &T::AccountId) -> bool {
			CandidatePool::<T>::contains_key(&validator)
		}


		fn check_delegated_amount(amount: BalanceOf<T>) -> DispatchResult {
			ensure!(amount >= T::MinDelegateAmount::get(), Error::<T>::BelowMinimumDelegateAmount);
			Ok(())
		}

		pub fn add_candidate_delegator(
			candidate: &T::AccountId,
			delegator: &T::AccountId,
		) -> DispatchResult {
			let mut candidate_delegators = CandidateDelegators::<T>::get(&candidate);
			candidate_delegators
				.try_push(delegator.clone())
				.map_err(|_| Error::<T>::TooManyDelegatorsInPool)?;
			CandidateDelegators::<T>::set(&candidate, candidate_delegators);
			Ok(())
		}

		fn increase_candidate_delegations(
			candidate: &T::AccountId,
			amount: &BalanceOf<T>,
		) -> DispatchResultWithValue<BalanceOf<T>> {
			let mut candidate_detail = Self::get_candidate(&candidate)?;
			let total_delegated_amount = candidate_detail.add_delegated_amount(*amount)?;
			CandidatePool::<T>::set(&candidate, Some(candidate_detail));

			Ok(total_delegated_amount)
		}

		fn decrease_candidate_delegations(
			candidate: &T::AccountId,
			amount: &BalanceOf<T>,
		) -> DispatchResultWithValue<BalanceOf<T>> {
			let mut candidate_detail = Self::get_candidate(&candidate)?;
			let total_delegated_amount = candidate_detail.sub_delegated_amount(*amount)?;
			CandidatePool::<T>::set(&candidate, Some(candidate_detail));

			Ok(total_delegated_amount)
		}
		/// Releasing the hold balance amount of candidate
		pub fn release_candidate_bonds(
			candidate: &T::AccountId,
			bond: BalanceOf<T>,
		) -> DispatchResult {
			T::NativeBalance::release(
				&HoldReason::CandidateBondReserved.into(),
				&candidate,
				bond,
				Precision::BestEffort,
			)?;
			Ok(())
		}

		/// Releasing the hold balance amount of delegator
		fn release_delegated_amount(
			delegator: &T::AccountId,
			amount: &BalanceOf<T>,
		) -> DispatchResult {
			T::NativeBalance::release(
				&HoldReason::DelegateAmountReserved.into(),
				&delegator,
				*amount,
				Precision::BestEffort,
			)?;
			Ok(())
		}
		fn remove_candidate_delegation_data(
			delegator: &T::AccountId,
			candidate: &T::AccountId,
		) -> DispatchResult {
			DelegationInfos::<T>::remove(&delegator, &candidate);

			let delegate_count = DelegateCountMap::<T>::get(&delegator);
			DelegateCountMap::<T>::set(&delegator, delegate_count.saturating_sub(1));

			// Remove delegator from the candidate delegators vector
			let mut candidate_delegators = CandidateDelegators::<T>::get(&candidate);
			candidate_delegators
				.binary_search(&delegator)
				.map_err(|_| Error::<T>::DelegationDoesNotExist)
				.map(|indx| candidate_delegators.remove(indx))?;
			CandidateDelegators::<T>::set(&candidate, candidate_delegators);

			Ok(())
		}

		pub(crate) fn select_validator_set() -> TopCandidateVec<T> {
			// If the number of candidates is below the threshold for active set, network won't
			// function
			if CandidatePool::<T>::count() < T::MinValidators::get() {
				return vec![];
			}
			let validator_len = T::MaxValidators::get();
			
			// Collect candidates with their total stake (bond + total delegations)
			let mut top_candidates: TopCandidateVec<T> = CandidatePool::<T>::iter()
				.map(|(candidate_id, candidate)| {
					let total_stake = candidate.total();
					(candidate_id, candidate.bond, total_stake)
				})
				.collect();

			// Sort candidates by their total stake in descending order
			top_candidates.sort_by_key(|&(_, _, total_stake)| Reverse(total_stake));

			// Select the top candidates based on the maximum active validators allowed
			let usize_validator_len = validator_len as usize;
			sorted_candidates.into_iter().take(validator_len).collect()
		}

		pub(crate) fn move_to_next_epoch(valivdator_set: TopCandidateVec<T>) {
			let epoch_index = EpochIndex::<T>::get();
			let next_epoch_index = epoch_index.saturating_add(1);
			EpochIndex::<T>::set(next_epoch_index);

			Self::deposit_event(Event::NextEpochMoved {
				last_epoch: epoch_index,
				next_epoch: next_epoch_index,
				at_block: frame::deps::frame_system::Pallet::<T>::block_number(),
				total_candidates: CandidatePool::<T>::count() as u64,
				total_validators: valivdator_set.len() as u64,
			});
		}

		pub fn report_new_validators(new_set: Vec<T::AccountId>) {
			T::ReportNewValidatorSet::report_new_validator_set(new_set);
		}

		pub fn capture_epoch_snapshot(
			validator_set: &CandidateDelegationSet<T>,
		) -> Epoch<T> {
			let mut epoch_snapshot = Epoch::<T>::default();
			for (validator_id, bond, _) in validator_set.to_vec().iter() {
				epoch_snapshot.add_validator(validator_id.clone(), bond.clone());
				for delegator in CandidateDelegators::<T>::get(validator_id) {
					if let Some(delegation_info) =
						DelegationInfos::<T>::get(&delegator, &validator_id)
					{
						epoch_snapshot.add_delegator(
							delegator,
							active_validator_id.clone(),
							delegation_info.amount,
						);
					}
				}
			}
			epoch_snapshot
		}

		fn execute_rewards() {
			if let Some(current_block_author) = Self::find_author() {
				if let Some(Epoch { validators, delegations }) = LastEpochSnapshot::<T>::get() {
					if let Some(total_bond) = validators.get(&current_block_author) {
						let bond = Percent::from_rational(5, 1000) * total;
						let mut rewards = Rewards::<T>::get(&validator);
						rewards = rewards.saturating_add(bond);
						Rewards::<T>::set(validator.clone(), rewards);
			
						for ((delegator, candidate), amount) in delegations.iter() {
							if candidate != validator {
								continue;
							}
							// Calculating the new reward of the block author
							let mut rewards = Rewards::<T>::get(&delegator);
							rewards = rewards.saturating_add(bond);
							Rewards::<T>::set(delegator, rewards);
						}						
					}
				}
			}
		}
	}
}