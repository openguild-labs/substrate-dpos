//!  # Delegated Proof of Stake (DPOS) Pallet
//!
//! The Substrate DPoS Pallet provides a Delegated Proof of Stake mechanism for a Substrate-based
//! blockchain. It allows token holders to delegate their tokens to validators who are responsible
//! for producing blocks and securing the network.
//!
//! ## Overview
//!
//! The DPoS pallet implements a governance mechanism where stakeholders can elect a set of
//! validators to secure the network. Token holders delegate their stake to validators, who then
//! participate in the block production process. This pallet includes functionality for delegating
//! stake, selecting validators, and handling rewards.
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
			tokens::Precision,
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
		collections::btree_set::BTreeSet
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

		/// The maximum number of validators that the pallet can hold.
		type MaxValidators: Get<u32>;

		/// The minimum number of validators that the pallet can hold.
		type MinValidators: Get<u32>;

		/// The maximum number of candidates that the pallet can hold.
		#[pallet::constant]
		type MaxCandidates: Get<u32>;

		/// The maximum number of delegators that the candidate can have
		/// If the number of delegators reaches the maximum, delegator with the lowest amount
		/// will be replaced by the new delegator if the new delegation is higher
		#[pallet::constant]
		type MaxCandidateDelegators: Get<u32>;

		/// The minimum amount that can be delegated to a candidate.
		#[pallet::constant]
		type MinDelegateAmount: Get<BalanceOf<Self>>;

		/// The minimum bond amount required to register as a candidate.
		#[pallet::constant]
		type MinCandidateBond: Get<BalanceOf<Self>>;
		
		/// The duration of an epoch in blocks.
		#[pallet::constant]
		type EpochDuration: Get<BlockNumberFor<Self>>;

		/// The maximum number of delegations that a delegator can have.
		#[pallet::constant]
		type MaxDelegateCount: Get<u32>;

		/// The origin which may forcibly create or destroy an item or otherwise alter privileged
		/// attributes.
		type ForceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		
		/// The reason for the pallet dpos placing a hold on funds.
		type RuntimeHoldReason: From<HoldReason>;
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
	
	/// The candidate pool stores the candidates along with their bond and total delegated amount.
	#[pallet::storage]
	pub type CandidatePool<T: Config> = CountedStorageMap<_, Twox64Concat, T::AccountId, Candidate<T>, OptionQuery>;
	/// The number of delegations that a delegator has.
	#[pallet::storage]
	pub type DelegateCountMap<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, u32, ValueQuery>;
	/// The delegations store the amount of tokens delegated by a delegator to a candidate.
	#[pallet::storage]
	pub type DelegationInfos<T: Config> = StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, T::AccountId, Delegation<T>, OptionQuery>;
	/// The candidate delegators store the delegators of a candidate.
	#[pallet::storage]
	pub type CandidateDelegators<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<T::AccountId, <T as Config>::MaxCandidateDelegators>, ValueQuery>;
	/// The current epoch index.
	#[pallet::storage]
	pub type EpochIndex<T: Config> = StorageValue<_, u32, ValueQuery>;
	/// The active validator set for the current epoch.
	#[allow(type_alias_bounds)]
	pub type TopCandidateVec<T: Config> = sp_std::vec::Vec<(T::AccountId, BalanceOf<T>, BalanceOf<T>)>;
	/// The active validator set for the current epoch.
	#[pallet::storage]
	#[pallet::getter(fn current_validators)]
	pub type CurrentValidators<T: Config> = StorageValue<_, BoundedVec<(T::AccountId, BalanceOf<T>, BalanceOf<T>), <T as Config>::MaxValidators>, ValueQuery>;

	/// Snapshot of the last epoch data, which includes the active validator set along with their
	/// total bonds and delegations. This storage is unbounded but safe, as it only stores `Vec`
	/// values within a `BoundedVec`. The total number of delegations is limited by the size
	/// `MaxValidators * MaxCandidateDelegators`.
	#[pallet::storage]
	#[pallet::unbounded]
	#[pallet::getter(fn last_epoch_snapshot)]
	pub type LastEpochSnapshot<T: Config> = StorageValue<_, Epoch<T>, OptionQuery>;

	/// Stores the total claimable rewards for each account, which can be a validator or a
	/// delegator. The reward points are updated with each block produced.
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

				let _ = T::NativeBalance::hold(&HoldReason::CandidateBondReserved.into(), &candidateId, *bond);
				let candidate = Candidate::new(*bond);
				CandidatePool::<T>::insert(&candidateId, candidate);
			}

			// Update the validator set using the data stored in the candidate pool
			let validator_set = Pallet::<T>::select_validator_set().to_vec();
			CurrentValidators::<T>::put(
				BoundedVec::try_from(validator_set.clone())
					.expect("Exceed limit number of the validators in the active set"),
			);
			// Capture the snapshot of the last epoch
			LastEpochSnapshot::<T>::set(Some(Pallet::<T>::capture_epoch_snapshot(
				&validator_set,
			)));

			// Report the new validator set to the runtime
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

	/// The pallet's dispatchable functions.
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// We execute the rewards calculation for last epoch block and the validator set selection logic at the start of
		/// each block.
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
		/// Event emitted when a candidate is removed from the candidate pool
		CandidateRegistrationRemoved { candidate_id: T::AccountId },
		/// Event emitted when a candidate is delegated by a delegator
		CandidateDelegated {
			candidate_id: T::AccountId,
			delegated_by: T::AccountId,
			amount: BalanceOf<T>,
			total_delegated_amount: BalanceOf<T>,
		},
		/// Event emitted when a candidate is undelegated by a delegator
		CandidateUndelegated {
			candidate_id: T::AccountId,
			delegator: T::AccountId,
			amount: BalanceOf<T>,
			left_delegated_amount: BalanceOf<T>,
		},
		/// Event emitted when the next epoch is moved
		NextEpochMoved {
			last_epoch: u32,
			next_epoch: u32,
			at_block: BlockNumberFor<T>,
			total_candidates: u64,
			total_validators: u64,
		},
		/// Event emitted when a reward is claimed
		RewardClaimed { claimer: T::AccountId, total_reward: BalanceOf<T> },
	}

	/// Errors inform users that something went wrong.
	/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error
	#[pallet::error]
	pub enum Error<T> {
		/// Thrown when there are too many validators exceeding the pool limit
		TooManyValidators,
		/// Thrown when the zero input amount is not accepted
		InvalidZeroAmount,
		/// Thrown when a delegator vote too many candidates exceeding the allowed limit
		TooManyCandidateDelegations,
		/// Thrown when candidate has too many delegations exceeding the delegator pool limit
		TooManyDelegatorsInPool,
		/// Thrown when the candidate already exists in the candidate pool
		CandidateAlreadyExist,
		/// Thrown when the candidate does not exist in the candidate pool
		CandidateDoesNotExist,
		/// Thrown when the delegator does not have any delegation with the candidate
		DelegationDoesNotExist,
		/// Thrown when the delegated amount is below the minimum amount
		BelowMinimumDelegateAmount,
		/// Thrown when the candidate bond is below the minimum amount
		BelowMinimumCandidateBond,
		/// Thrown when there is no claimable reward found
		NoClaimableRewardFound,
		/// Thrown when the candidate has too many delegations exceeding the allowed limit
		InvalidMinimumDelegateAmount,
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

		/// Nodes can register themselves as a candidate in the DPoS (Delegated Proof of Stake)
		/// network.
		///
		/// Requires the caller to provide a bond amount greater than zero and at least equal to the
		/// minimum required candidate bond configured in the pallet's runtime configuration
		/// (`MinCandidateBond`).
		///
		/// If successful, the caller's account is registered as a candidate with the specified bond
		/// amount, and a `CandidateRegistered` event is emitted.
		///
		/// Parameters:
		/// - `origin`: The origin of the transaction.
		/// - `bond`: The amount of funds to bond as part of the candidate registration.
		///
		/// Errors:
		/// - `InvalidZeroAmount`: Raised if `bond` is zero.
		/// - `BelowMinimumCandidateBond`: Raised if `bond` is less than `MinCandidateBond`.
		/// - `CandidateAlreadyExist`: Raised if the caller is already registered as a candidate.
		///
		/// Emits:
		/// - `CandidateRegistered`: When a candidate successfully registers, including the
		///   candidate's account ID (`candidate_id`) and the initial bond amount (`initial_bond`).
		///
		pub fn register_as_candidate(
			origin: OriginFor<T>,
			initial_bond: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure the bond amount is greater than zero and at least equal to the minimum required
			ensure!(initial_bond > Zero::zero(), Error::<T>::InvalidZeroAmount);
			ensure!(initial_bond >= T::MinCandidateBond::get(), Error::<T>::BelowMinimumCandidateBond);

			let who = ensure_signed(origin)?;
			ensure!(!Self::is_candidate(&who), Error::<T>::CandidateAlreadyExist);
			ensure!(
				CandidatePool::<T>::count().saturating_add(1) <= T::MaxCandidates::get(),
				Error::<T>::TooManyValidators
			);

			// Only hold the funds of a user which has no holds already.
			T::NativeBalance::hold(&HoldReason::CandidateBondReserved.into(), &who, initial_bond)?;

			// Register the candidate in the candidate pool
			let candidate = Candidate::new(initial_bond);
			CandidatePool::<T>::insert(&who, candidate);
			// Emit an event to notify that the candidate has been registered
			Self::deposit_event(Event::CandidateRegistered { candidate_id: who, initial_bond });
			Ok(())
		}

		/// Delegates a specified amount of funds to a candidate in the DPoS (Delegated Proof of
		/// Stake) network.
		///
		/// Requires the caller to provide an amount greater than zero.
		///
		/// If the delegator has previously delegated to the candidate, the delegated amount is
		/// updated by adding the new amount to the existing delegation. If it's the first time
		/// delegation, a new delegation record is initialized.
		///
		/// Parameters:
		/// - `origin`: The origin of the transaction.
		/// - `candidate`: The account ID of the candidate to delegate funds to.
		/// - `amount`: The amount of funds to delegate.
		///
		/// Errors:
		/// - `InvalidZeroAmount`: Raised if `amount` is zero.
		/// - `TooManyCandidateDelegations`: Raised if the delegator exceeds the maximum allowed
		///   number of candidate delegations.
		/// - `BalanceOverflow`: Raised if adding `amount` to an existing delegated amount results
		///   in overflow.
		///
		/// Effects:
		/// - Updates the delegated amount for the specified candidate and delegator.
		/// - Increases the count of candidates delegated to by the delegator if it's the first time
		///   delegating to this candidate.
		/// - Holds `amount` from the delegator's account as delegated amount.
		///
		/// Emits:
		/// - `CandidateDelegated`: When a delegator successfully delegates funds to a candidate,
		///   including the candidate's account ID (`candidate_id`), delegator's account ID
		///   (`delegated_by`), the delegated amount (`amount`), and the total delegated amount to
		///   the candidate after the delegation (`total_delegated_amount`).
		///
		pub fn delegate(
			origin: OriginFor<T>,
			candidate: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure!(amount > Zero::zero(), Error::<T>::InvalidZeroAmount);
			let delegator = ensure_signed(origin)?;
			match DelegationInfos::<T>::try_get(&delegator, &candidate) {
				// If the delegator has previously delegated to the candidate, update the delegated amount
				Ok(mut delegation_info) => {
					// Check if the new delegated amount will overflow
					let new_delegated_amount =
						delegation_info.amount.checked_add(&amount).expect("Overflow");
					Self::check_delegated_amount(new_delegated_amount)?;
					// Update the delegated amount
					delegation_info.set_amount(new_delegated_amount);
					DelegationInfos::<T>::set(&delegator, &candidate, Some(delegation_info));
				},
				Err(_) => {
					// If it's the first time delegation, initialize a new delegation record
					// Check if the new delegated amount will overflow
					Self::check_delegated_amount(amount)?;
					let delegate_count = DelegateCountMap::<T>::get(&delegator);
					let new_delegate_count = delegate_count.saturating_add(1);
					ensure!(
						new_delegate_count <= T::MaxDelegateCount::get(),
						Error::<T>::TooManyCandidateDelegations
					);
					// Update the delegator's delegate count
					DelegateCountMap::<T>::set(&delegator, new_delegate_count);
					// Update the candidate's delegator list
					Self::add_candidate_delegator(&candidate, &delegator)?;
					// Initialize a new delegation record
					let new_delegation_info = Delegation::new(amount);
					// Set the new delegation record
					DelegationInfos::<T>::insert(&delegator, &candidate, new_delegation_info);
				},
			};
			// Hold the delegated amount from the delegator's account
			T::NativeBalance::hold(&HoldReason::DelegateAmountReserved.into(), &delegator, amount)?;

			// Increase the candidate's total delegated amount
			let total_delegated_amount = Self::increase_candidate_delegations(&candidate, &amount)?;
			// Emit an event to notify that the candidate has been delegated
			Self::deposit_event(Event::CandidateDelegated {
				candidate_id: candidate,
				delegated_by: delegator,
				amount,
				total_delegated_amount,
			});
			Ok(())
		}

		/// Unregisters a candidate from the DPoS (Delegated Proof of Stake) network.
		///
		/// Requires the caller to have the privilege defined by `ForceOrigin`.
		///
		/// Parameters:
		/// - `origin`: The origin of the transaction, which must be authorized by `ForceOrigin`.
		/// - `candidate`: The account ID of the candidate to be deregistered.
		///
		/// Errors:
		/// - `CandidateDoesNotExist`: Raised if the candidate specified does not exist in the
		///   candidate pool.
		///
		/// Effects:
		/// - Deregisters the candidate identified by `candidate` from the candidate pool.
		///
		/// Emits:
		/// - `CandidateRegistrationRemoved`: When a candidate is successfully removed from the
		///  candidate pool, including the candidate's account ID (`candidate_id`).
		/// - `RewardClaimed`: When a candidate claims the reward, including the claimer's account ID
		/// (`claimer`) and the total reward claimed (`total_reward`).
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
			CandidateDelegators::<T>::remove(&candidate);

			// Releasing the hold bonds of the candidate
			let candidate_detail = Self::get_candidate(&candidate)?;
			Self::release_candidate_bonds(&candidate, candidate_detail.bond)?;
			// Claiming the rewards of the candidate
			let rewards = Rewards::<T>::get(&candidate);
			if rewards > Zero::zero() {
				// Mint the rewards to the candidate
				let _ = T::NativeBalance::mint_into(&candidate, rewards);
				// Remove the rewards from the storage
				Rewards::<T>::remove(&candidate);
				Self::deposit_event(Event::RewardClaimed { claimer: candidate.clone(), total_reward: rewards });
			}
			// Removing any information related the registration of the candidate in the pool
			CandidatePool::<T>::remove(&candidate);

			Self::deposit_event(Event::CandidateRegistrationRemoved { candidate_id: candidate.clone() });

			Ok(())
		}

		/// Undelegates a specified amount of funds from a candidate in the DPoS
		/// (Delegated Proof of Stake) network.
		///
		/// Requires the caller to have the privilege defined by `ForceOrigin`.
		///
		/// Parameters:
		/// - `origin`: The origin of the transaction, which must be authorized by `ForceOrigin`.
		/// - `delegator`: The account ID of the delegator who wants to undelegate funds.
		/// - `candidate`: The account ID of the candidate from whom funds will be undelegated.
		/// - `amount`: The amount of funds to undelegate.
		///
		/// Errors:
		/// - `CandidateDoesNotExist`: Raised if the specified candidate does not exist in the
		///   candidate pool.
		/// - Errors from `undelegate_candidate_inner` function, such as insufficient funds to
		///   undelegate.
		///
		/// Effects:
		/// - Undelegates the specified `amount` of funds from the `candidate` by the `delegator`.
		///
		/// Emits:
		/// - `CandidateUndelegated`: When a delegator successfully undelegates funds from a candidate,
		///  including the candidate's account ID (`candidate_id`), the delegator's account ID
		/// (`delegator`), the undelegated amount (`amount`), and the remaining delegated amount to
		/// the candidate after the undelegation (`left_delegated_amount`).
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

		/// Claims the accumulated reward points as native tokens for the claimer (validator or
		/// delegator) in the DPoS (Delegated Proof of Stake) network.
		///
		/// Parameters:
		/// - `origin`: The origin of the transaction, which must be signed by the claimer
		///   (validator or delegator).
		///
		/// Errors:
		/// - If no claimable rewards are found for the claimer, the function will return an
		///   `Error`.
		///
		/// Effects:
		/// - Mints native tokens into the claimer's account equivalent to their accumulated reward
		///   points.
		/// - Removes the claimer's accumulated reward points from storage after claiming.
		/// - Emits a `RewardClaimed` event upon successful claim.
		/// 
		/// Emits:
		/// - `RewardClaimed`: When a claimer successfully claims their reward, including the
		///  claimer's account ID (`claimer`) and the total reward claimed (`total_reward`).
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

	/// implementation of some util function the pallet
	impl<T: Config> Pallet<T> {
		/// Get the delegation information between a delegator and a candidate.
		pub fn get_delegation(
			delegator: &T::AccountId,
			candidate: &T::AccountId,
		) -> DispatchResultWithValue<Delegation<T>> {
			Ok(DelegationInfos::<T>::try_get(&delegator, &candidate)
				.map_err(|_| Error::<T>::DelegationDoesNotExist)?)
		}

		/// Get the candidate information from the candidate pool.
		pub fn get_candidate(
			candidate: &T::AccountId,
		) -> DispatchResultWithValue<Candidate<T>> {
			Ok(CandidatePool::<T>::try_get(&candidate)
				.map_err(|_| Error::<T>::CandidateDoesNotExist)?)
		}

		/// Check if the candidate is in the candidate pool.
		pub fn is_candidate(validator: &T::AccountId) -> bool {
			CandidatePool::<T>::contains_key(&validator)
		}

		/// Check if the delegator has delegated to the candidate.
		fn check_delegated_amount(amount: BalanceOf<T>) -> DispatchResult {
			ensure!(amount >= T::MinDelegateAmount::get(), Error::<T>::BelowMinimumDelegateAmount);
			Ok(())
		}

		/// Add a delegator to the candidate's delegators list.
		pub fn add_candidate_delegator(
			candidate: &T::AccountId,
			delegator: &T::AccountId,
		) -> DispatchResult {
			// Add the delegator to the candidate's delegators list
			let mut candidate_delegators = CandidateDelegators::<T>::get(&candidate);
			candidate_delegators
				.try_push(delegator.clone())
				.map_err(|_| Error::<T>::TooManyDelegatorsInPool)?;
			// Update the candidate's delegators list
			CandidateDelegators::<T>::set(&candidate, candidate_delegators);
			Ok(())
		}
		
		/// Increase the total delegated amount of the candidate.
		fn increase_candidate_delegations(
			candidate: &T::AccountId,
			amount: &BalanceOf<T>,
		) -> DispatchResultWithValue<BalanceOf<T>> {
			// Increase the candidate's total delegated amount
			let mut candidate_detail = Self::get_candidate(&candidate)?;
			let total_delegated_amount = candidate_detail.add_delegated_amount(*amount)?;
			// Update the candidate's total delegated amount
			CandidatePool::<T>::set(&candidate, Some(candidate_detail));

			Ok(total_delegated_amount)
		}

		/// Decrease the total delegated amount of the candidate.
		fn decrease_candidate_delegations(
			candidate: &T::AccountId,
			amount: &BalanceOf<T>,
		) -> DispatchResultWithValue<BalanceOf<T>> {
			let mut candidate_detail = Self::get_candidate(&candidate)?;
			// Decrease the candidate's total delegated amount
			let total_delegated_amount = candidate_detail.sub_delegated_amount(*amount)?;
			CandidatePool::<T>::set(&candidate, Some(candidate_detail));

			Ok(total_delegated_amount)
		}

		/// Releasing the hold balance amount of candidate
		pub fn release_candidate_bonds(
			candidate: &T::AccountId,
			bond: BalanceOf<T>,
		) -> DispatchResult {
			// Releasing the hold balance amount of candidate
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

		/// Remove the delegation information between a delegator and a candidate.
		fn remove_candidate_delegation_data(
			delegator: &T::AccountId,
			candidate: &T::AccountId,
		) -> DispatchResult {
			// Remove the delegation information between the delegator and the candidate
			DelegationInfos::<T>::remove(&delegator, &candidate);
			// Decrease the delegator's delegate count
			let delegate_count = DelegateCountMap::<T>::get(&delegator);
			DelegateCountMap::<T>::set(&delegator, delegate_count.saturating_sub(1));

			// Remove delegator from the candidate delegators vector
			let mut candidate_delegators = CandidateDelegators::<T>::get(&candidate);
			// find the delegator in the candidate delegators list and remove it
			candidate_delegators
				.binary_search(&delegator)
				.map_err(|_| Error::<T>::DelegationDoesNotExist)
				.map(|indx| candidate_delegators.remove(indx))?;
			CandidateDelegators::<T>::set(&candidate, candidate_delegators);

			Ok(())
		}

		/// Select the validator set for the next epoch.
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
			top_candidates.into_iter().take(usize_validator_len).collect()
		}

		/// Move to the next epoch.
		pub(crate) fn move_to_next_epoch(valivdator_set: TopCandidateVec<T>) {
			// Increment the epoch index
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

		/// Report the new validator set to the runtime.
		pub fn report_new_validators(new_set: Vec<T::AccountId>) {
			T::ReportNewValidatorSet::report_new_validator_set(new_set);
		}

		/// Capture the snapshot of the current epoch.
		pub fn capture_epoch_snapshot(
			validator_set: &TopCandidateVec<T>,
		) -> Epoch<T> {
			let mut epoch_snapshot = Epoch::<T>::default();
			// Add the validators and their total bond to the snapshot
			for (validator_id, bond, _) in validator_set.to_vec().iter() {
				epoch_snapshot.add_validator(validator_id.clone(), bond.clone());
				// Add the delegators and their delegated amount to the snapshot
				for delegator in CandidateDelegators::<T>::get(validator_id) {
					if let Some(delegation_info) =
						DelegationInfos::<T>::get(&delegator, &validator_id)
					{
						epoch_snapshot.add_delegator(
							delegator,
							validator_id.clone(),
							delegation_info.amount,
						);
					}
				}
			}
			// Return the snapshot of the current epoch
			epoch_snapshot
		}

		/// Execute the rewards calculation for the last epoch block.
		fn execute_rewards() {
			// Get the current block author
			if let Some(current_block_author) = Self::find_author() {
				// Get the snapshot of the last epoch
				if let Some(Epoch { validators, delegations }) = LastEpochSnapshot::<T>::get() {
					// Calculate the rewards for the block author and the delegators
					if let Some(total_bond) = validators.get(&current_block_author) {
						// Calculating the new reward of the block author
						// The reward is calculated as 5% of the total bond of the block author
						// The reward is distributed to the block author and the delegators
						// based on the amount delegated to the block author.						
						let bond = Percent::from_rational(5 as u32, 100) * Percent::from_rational(1000 as u32, 1000) * *total_bond;
						let mut rewards = Rewards::<T>::get(&current_block_author);
						rewards = rewards.saturating_add(bond);
						Rewards::<T>::set(current_block_author.clone(), rewards);
						// Calculate the rewards for the delegators
						// The reward is calculated as 5% of the total bond of the block author
						// The reward is distributed to the block author and the delegators
						// based on the amount delegated to the block author.
						for ((delegator, candidate), amount) in delegations.iter() {
							if *candidate != current_block_author {
								continue;
							}
							// Calculating the new reward of the block author
							let bond = Percent::from_rational(5 as u32, 100) * Percent::from_rational(1000 as u32, 1000) * *amount;
							let mut rewards = Rewards::<T>::get(&delegator);
							rewards = rewards.saturating_add(bond);
							// Update the rewards for the delegator
							Rewards::<T>::set(delegator, rewards);
						}						
					}
				}
			}
		}
	}
}