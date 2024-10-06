# Substrate Delegated Proof of Stake blockchain

Delegated Proof of Stake (DPoS) is a blockchain consensus mechanism where network users vote and elect delegates to validate the next block.

# Table of Contents

- [Substrate Delegated Proof of Stake blockchain](#substrate-delegated-proof-of-stake-blockchain)
- [Table of Contents](#table-of-contents)
  - [Introduction](#introduction)
  - [General Definitions](#general-definitions)
  - [Prerequisites](#prerequisites)
  - [Setup local machine](#setup-local-machine)
  - [Walkthrough this github](#walkthrough-this-github)
    - [Pallet structure folder](#pallet-structure-folder)
    - [Candidate and Delegator](#candidate-and-delegator)
    - [Select candidates to validators in each block epoch](#select-candidates-to-validators-in-each-block-epoch)
      - [Genesis](#genesis)
      - [Validator Election](#validator-election)
      - [Rewards](#rewards)
      - [Find author the block and next to next epoch](#find-author-the-block-and-next-to-next-epoch)
    - [Runtime](#runtime)
  - [How to build this course](#how-to-build-this-course)
      - [Using `omni-node`](#using-omni-node)
  - [How to run `omni-node`?](#how-to-run-omni-node)
  - [References](#references)

## Introduction

  Staking refers to the process of participating in the network's consensus mechanism to help secure the network and validate transactions.

  The candidates (nodes that produce blocks) are selected based on their stake in the network. And here is where staking comes in.

Candidates (and token holders if they delegate) have a stake in the network. The top N candidates by staked amount are chosen to produce blocks with a valid set of transactions, where N is a configurable parameter. Part of each block reward goes to the active set of candidates that produced the block, who then shares it with the delegators considering their percental contributions towards the candidates's stake. In such a way, network members are incentivized to stake tokens to improve the overall security. Since staking is done at a protocol level through the staking interface, if you choose to delegate, the candidates you delegate do not have access to your tokens.

## General Definitions

- `Candidates`: node operators that are eligible to become block producers if they can acquire enough stake to be in the active set
- `Delegator`: token holders who stake tokens, vouching for specific candidates. Any user that holds a minimum amount of tokens as free balance can become a delegator
- `Delegating`: A process of the delegator to vote for the candidate for the next epoch's validator election using tokens.
- `Minimum delegation per candidate`: minimum amount of tokens to delegate candidates once a user is in the set of delegators
- `Maximum delegators per candidate`: maximum number of delegators, by staked amount, that a candidate can have which are eligible to receive staking rewards
- `Maximum delegations`: maximum number of candidates a delegator can delegate
- `Commission`: The percentage that block author and its delegator receive for a successfully produced block.
- `Slash`: The punishment of an active validator if they misbehave.
- `Epoch`: A predefined period during which the set of active validators remains fixed. At the end of each epoch, a new set of validators can be elected based on the current delegations.
- `Bond`: Staked tokens are bonded, meaning they are locked for a certain period, which secures the network and aligns incentives.

## Prerequisites

This requires you to finish a first few tutorials of Substrate development from the official documentation. If you have not walked through those first. Please take a look at these first before diving deeper into this interactive tutorial:

- [TheLowLevelers - Run a local Substrate Node (Vietnamese)](https://lowlevelers.com/blog/polkadot/polkadot-guide-chay-local-substrate-node)
- [Substrate Tutorial - Build a local blockchain](https://docs.substrate.io/tutorials/build-a-blockchain/build-local-blockchain/)
- [Substrate Tutorial - Pallet](https://docs.substrate.io/tutorials/build-application-logic/)

## Setup local machine

If your hardware is a modern M1 Apple sillicon chip, working with Substrate can be very painful because there is many unstable compilation issue happens during your development. To avoid this, please install Rust toolchain following these versions below.

```
❯ cargo --version
cargo 1.76.0-nightly (71cd3a926 2023-11-20)
❯ rustc --version
rustc 1.76.0-nightly (3a85a5cfe 2023-11-20)
❯ rustup --version
rustup 1.25.2 (17db695f1 2023-02-01)
```

## Walkthrough this github

We have total 8 steps (maybe more).

### Pallet structure folder

The FRAME development environment provides modules—called pallets—and support libraries that you can use, modify, and extend to build the runtime logic to suit the needs of your blockchain.

A FRAME pallet is comprised of a number of blockchain primitives, including:

- Storage: FRAME defines a rich set of powerful [storage
  abstractions](https://docs.substrate.io/build/runtime-storage/) that makes it
  easy to use Substrate's efficient key-value database to manage the evolving
  state of a blockchain.
- Dispatchables: FRAME pallets define special types of functions that can be
  invoked (dispatched) from outside of the runtime in order to update its state.
- Events: Substrate uses
  [events](https://docs.substrate.io/build/events-and-errors/) to notify users
  of significant state changes.
- Errors: When a dispatchable fails, it returns an error.

Each pallet has its own `Config` trait which serves as a configuration interface
to generically define the types and parameters it depends on.

### Candidate and Delegator

In this course, I will introduce the simple dpos. The candidates can register to be validator, and delegator can stake some bond to candidate.
We have some event:

- `CandidateRegistered`: Event emitted when there is a new candidate registered
- `CandidateRegistrationRemoved`: Event emitted when candidate is removed from the candidate pool
- `CandidateDelegated`: Event emitted when candidate is delegated
- `CandidateUndelegated`: Event emitted when candidate is delegated
```rust
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
```
And some functions:
- `register_as_candidate`: Allows a node to register itself as a candidate in the DPOS network.
- `delegate`: Allows a delegator to delegate tokens to a candidate.
- `unregister_as_candidate`: unregisters a candidate from the DPoS (Delegated Proof of Stake) network.
- `undelegate`: undelegates a specified amount of funds from a candidate in the DPoS (Delegated Proof of Stake) network.
```rust
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
			CandidateDelegators::<T>::remove(&candidate);

			// Releasing the hold bonds of the candidate
			let candidate_detail = Self::get_candidate(&candidate)?;
			Self::release_candidate_bonds(&candidate, candidate_detail.bond)?;
			let rewards = Rewards::<T>::get(&candidate);
			if rewards > Zero::zero() {
				let _ = T::NativeBalance::mint_into(&candidate, rewards);
				Rewards::<T>::remove(&candidate);
				Self::deposit_event(Event::RewardClaimed { claimer: candidate.clone(), total_reward: rewards });
			}
			// Removing any information related the registration of the candidate in the pool
			CandidatePool::<T>::remove(&candidate);

			Self::deposit_event(Event::CandidateRegistrationRemoved { candidate_id: candidate.clone() });

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
```

### Select candidates to validators in each block epoch

#### Genesis

- Read through [Genesis Config](https://docs.substrate.io/build/genesis-configuration/). For simple explanation, the first block produced by any blockchain is referred to as the genesis block. The hash associated with this block is the top-level parent of all blocks produced after that first block.
  
```rust
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
```

- At block building state, we build candidate pool storage, and select top validators by total stake amount.
- And capture current information of this block, set new validator to the runtime config.
```rust
pub fn capture_epoch_snapshot(
			validator_set: &TopCandidateVec<T>,
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
							validator_id.clone(),
							delegation_info.amount,
						);
					}
				}
			}
			epoch_snapshot
		}
```
#### Validator Election

- Top validators under `MaxValidators` and above `MinValidators` are selected based on the total amount of delegated amount and the total amount they bonded.
- If there is not enough validators (under the configured `MinValidators`), the active validator set is empty. By this way, there is no block produced and no reward distributed.
- In this pallet, the top validators will be sorted out and selected at the beginning of the new epoch.
```rust
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
```
  #### Rewards

- When prepare to next epoch block, we calculate an amount of bond for rewarding. For simple, we choose a formula: 5% of total staking amount. And add this bond to reward storage of each delegator and validator.
- You should read [Hooks](https://paritytech.github.io/polkadot-sdk/master/frame_support/traits/trait.Hooks.html#summary). On this epoch block, we calculate rewards in last epoch block.
  
  ``` rust
    fn execute_rewards() {
   if let Some(current_block_author) = Self::find_author() {
    if let Some(Epoch { validators, delegations }) = LastEpochSnapshot::<T>::get() {
     if let Some(total_bond) = validators.get(&current_block_author) {
      // in this simple version, I setup hard the author commission is 5%
      let bond = Percent::from_rational(5 as u32, 100) * Percent::from_rational(1000 as u32, 1000) * *total_bond;
      let mut rewards = Rewards::<T>::get(&current_block_author);
      rewards = rewards.saturating_add(bond);
      Rewards::<T>::set(current_block_author.clone(), rewards);
   
      for ((delegator, candidate), amount) in delegations.iter() {
       if *candidate != current_block_author {
        continue;
       }
       // Calculating the new reward of the block author
      // in this simple version, I setup hard the delegator commission is 5%
       let bond = Percent::from_rational(5 as u32, 100) * Percent::from_rational(1000 as u32, 1000) * *amount;
       let mut rewards = Rewards::<T>::get(&delegator);
       rewards = rewards.saturating_add(bond);
       Rewards::<T>::set(delegator, rewards);
      }      
     }
    }
   }
  }
  ```

- After epoch block, I get sum of bonds of ths block author and calculate the rewards additions. Loop all delegation of the author, and calculate the rewards of each delegations.
- We have an event: `RewardClaimed`. When Delegator undelegate, we will trigger this event and deposit reward to delegator.

#### Find author the block and next to next epoch
- Prepare to go to next epoch block, We must to find author of current block and calculate the rewards for the author and the delegators.
```rust
	  /// Find the author of a block. A fake provide for this type is provided in the runtime. You
		/// can use a similar mechanism in your tests.
		type FindAuthor: FindAuthor<Self::AccountId>;
```

- In this course, we can use simple version to find author at runtime.
```rust
pub struct RoundRobinAuthor;
impl FindAuthor<AccountId> for RoundRobinAuthor {
	fn find_author<'a, I>(_: I) -> Option<AccountId>
	where
		I: 'a + IntoIterator<Item = ([u8; 4], &'a [u8])>,
	{
		let active_validator_ids = ValidatorSet::get();
		if active_validator_ids.len() == 0 {
			return None;
		}
		active_validator_ids
			.get((System::block_number() % (active_validator_ids.len() as u32)) as usize)
			.cloned()
	}
}
```
- Just get block number and we find the modular with length of validators set of current block.
-  We increase epoch index by 1, and trigger NextEpochMoved event
  
```rust
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
```
### Runtime

- We build [a genesis config](https://docs.substrate.io/build/genesis-configuration/). In Substrate, the terms "runtime" and "state transition function" are analogous.
Both terms refer to the core logic of the blockchain that is responsible for
validating blocks and executing the state changes they define. The Substrate project in this repository uses [FRAME](https://docs.substrate.io/learn/runtime-development/#frame) to construct a blockchain runtime. FRAME allows runtime developers to declare domain-specific logic in modules called "pallets". At the heart of FRAME is a helpful [macro language](https://docs.substrate.io/reference/frame-macros/) that makes it easy to create pallets and flexibly compose them to create blockchains that can address [a variety of needs](https://substrate.io/ecosystem/projects/).

Review the [FRAME runtime implementation](./runtime/src/lib.rs) included in this
template and note the following:

- This file configures several pallets to include in the runtime. Each pallet
  configuration is defined by a code block that begins with `impl $PALLET_NAME::Config for Runtime`.
- The pallets are composed into a single runtime by way of the
  [`construct_runtime!`](https://paritytech.github.io/substrate/master/frame_support/macro.construct_runtime.html) macro, which is part of the [core FRAME pallet library](https://docs.substrate.io/reference/frame-pallets/#system-pallets).
```rust
parameter_types! {
	pub const MaxCandidates : u32 = 200;
	pub const MaxCandidateDelegators : u32 = 300;
	pub const MinCandidateBond: u32 = 1_000;
	pub const MaxActivevalidators: u32 = 100;
	pub const MinActiveValidators: u32 = 3;
	pub const MaxDelegateCount : u32 = 30;
	pub const EpochDuration : u32 = EPOCH_DURATION;
	pub const MinDelegateAmount : u128 = 150;
}

pub struct RoundRobinAuthor;
impl FindAuthor<AccountId> for RoundRobinAuthor {
	fn find_author<'a, I>(_: I) -> Option<AccountId>
	where
		I: 'a + IntoIterator<Item = ([u8; 4], &'a [u8])>,
	{
		let active_validator_ids = ValidatorSet::get();
		if active_validator_ids.len() == 0 {
			return None;
		}
		active_validator_ids
			.get((System::block_number() % (active_validator_ids.len() as u32)) as usize)
			.cloned()
	}
}

parameter_types! {
	// This is a temporary storage that will keep the validators. In reality, this would have been
	// `pallet-aura` or another pallet that would consume these.
	pub storage ValidatorSet: Vec<AccountId> = vec![];
}

pub struct StoreNewValidatorSet;
impl pallet_dpos::ReportNewValidatorSet<AccountId> for StoreNewValidatorSet {
	fn report_new_validator_set(new_set: Vec<AccountId>) {
		ValidatorSet::set(&new_set);
	}
}

/// Configure the pallet-dpos in pallets/dpos.
impl pallet_dpos::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type NativeBalance = Balances;
	type MaxCandidates = MaxCandidates;
	type MaxCandidateDelegators = MaxCandidateDelegators;
	type MaxValidators = MaxActivevalidators;
	type MinValidators = MinActiveValidators;
	type ReportNewValidatorSet = StoreNewValidatorSet;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MaxDelegateCount = MaxDelegateCount;
	type EpochDuration = EpochDuration;
	type MinCandidateBond = MinCandidateBond;
	type MinDelegateAmount = MinDelegateAmount;
	type FindAuthor = RoundRobinAuthor;
	type ForceOrigin = EnsureRoot<AccountId>;
}
```
## How to build this course
#### Using `omni-node`

First, make sure to install the special omni-node of the PBA assignment, if you have not done so
already from the previous activity.

```sh
cargo install --force --git https://github.com/kianenigma/pba-omni-node.git
```

Then, you have two options:

1. Run with the default genesis using the `--runtime` flag:

```sh
cargo build --release
pba-omni-node --runtime ./target/release/wbuild/pba-runtime/pba_runtime.wasm --tmp
```

2. Run with a more flexible genesis using the `--chain` flag:

```sh
cargo install staging-chain-spec-builder
```

Feel free to populate your chain-spec file then with more accounts, like:

```json
{
  "dpos": {
    "genesisCandidates": [
      ["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", 100000],
      ["5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty", 100000],
      ["5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y", 100000],
      ["5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy", 100000],
      ["5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw", 100000],
      ["5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL", 100000],
      ["5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY", 100000],
      ["5HpG9w8EBLe5XCrbczpwq5TSXvedjrBGCwqxK1iQ7qUsSWFc", 100000],
      ["5Ck5SLSHYac6WFt5UZRSsdJjwmpSZq85fd5TRNAdZQVzEAPT", 100000],
      ["5HKPmK9GYtE1PSLsS1qiYU9xQ9Si1NcEhdeCq9sw5bqu4ns8", 100000],
      ["5FCfAonRZgTFrTd9HREEyeJjDpT397KMzizE6T3DvebLFE7n", 100000],
      ["5CRmqmsiNFExV6VbdmPJViVxrWmkaXXvBrSX8oqBT8R9vmWk", 100000],
      ["5Fxune7f71ZbpP2FoY3mhYcmM596Erhv1gRue4nsPwkxMR4n", 100000],
      ["5CUjxa4wVKMj3FqKdqAUf7zcEMr4MYAjXeWmUf44B41neLmJ", 100000]
    ]
  }
}
```
Add this to your `chain_spec.json`

```md
cd ./runtime

# Build the runtime

cargo build --release

# Generate chain-spec

chain-spec-builder create --chain-name DPOS -r ../target/release/wbuild/pba-runtime/pba_runtime.wasm default
```

## How to run `omni-node`?

```
pba-omni-node --chain ./runtime/chain_spec.json --tmp
```
## References

- Solo-chain-template: <https://github.com/paritytech/polkadot-sdk-solochain-template/tree/master>
- Polkadot academy material: <https://github.com/Polkadot-Blockchain-Academy/pba-content/tree/main/syllabus/6-Polkadot-SDK>
- Shout out Chase Chung for supporting me complete this course: <https://github.com/chungquantin/substrate-dpos>
