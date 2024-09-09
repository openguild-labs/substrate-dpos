# Substrate Delegated Proof of Stake blockchain
Delegated Proof of Stake (DPoS) is a blockchain consensus mechanism where network users vote and elect delegates to validate the next block.
# Table of Contents
- [Substrate Delegated Proof of Stake blockchain](#substrate-delegated-proof-of-stake-blockchain)
- [Table of Contents](#table-of-contents)
  - [Introduction](#introduction)
  - [General Definitions](#general-definitions)
  - [Prerequisites](#prerequisites)
  - [Setup local machine](#setup-local-machine)
  - [Pallet structure folder](#pallet-structure-folder)
  - [Candidate and Delegator](#candidate-and-delegator)
  - [Select candidates to validators in each block epoch](#select-candidates-to-validators-in-each-block-epoch)
      - [Validator Election](#validator-election)
  - [How to use this cousre](#how-to-use-this-cousre)
  - [Walkthrough this github](#walkthrough-this-github)
    - [Pallet structure folder](#pallet-structure-folder-1)
    - [Candidate and Delegator](#candidate-and-delegator-1)
    - [Select candidates to validators in each block epoch](#select-candidates-to-validators-in-each-block-epoch-1)
      - [Validator Election](#validator-election-1)
      - [Rewards](#rewards)
    - [Runtime](#runtime)
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


## Pallet structure folder
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
## Candidate and Delegator
In this course, I will introduce the simple dpos. The candidates can register to be validator, and delegator can stake some bond to candidate.
We have some event:
- `CandidateRegistered`: Event emitted when there is a new candidate registered
- `CandidateRegistrationRemoved`: Event emitted when candidate is removed from the candidate pool
- `CandidateDelegated`: Event emitted when candidate is delegated
- `CandidateUndelegated`: Event emitted when candidate is delegated
And some functions:
- `register_as_candidate`: Allows a node to register itself as a candidate in the DPOS network.
- `delegate`: Allows a delegator to delegate tokens to a candidate.
- `unregister_as_candidate`: unregisters a candidate from the DPoS (Delegated Proof of Stake) network.
- `undelegate`: undelegates a specified amount of funds from a candidate in the DPoS (Delegated Proof of Stake) network.
## Select candidates to validators in each block epoch
- When new block produces, we will snapshot and choose the top candidates to be validators. We use [hooks](https://paritytech.github.io/polkadot-sdk/master/frame_support/pallet_macros/attr.hooks.html) to handle the snapshot event.
- event:
  `NextEpochMoved`
  #### Validator Election

- Top validators under `MaxValidators` and above `MinValidators` are selected based on the total amount of delegated amount and the total amount they bonded.
- If there is not enough validators (under the configured `MinValidators`), the active validator set is empty. By this way, there is no block produced and no reward distributed.
- In this pallet, the top validators will be sorted out and selected at the beginning of the new epoch.
=======
## How to use this cousre

Check branch in this github. I organize stes by steps to help you be familiar and follow easily.

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
And some functions:
- `register_as_candidate`: Allows a node to register itself as a candidate in the DPOS network.
- `delegate`: Allows a delegator to delegate tokens to a candidate.
- `unregister_as_candidate`: unregisters a candidate from the DPoS (Delegated Proof of Stake) network.
- `undelegate`: undelegates a specified amount of funds from a candidate in the DPoS (Delegated Proof of Stake) network.
### Select candidates to validators in each block epoch
- When new block produces, we will snapshot and choose the top candidates to be validators. We use [hooks](https://paritytech.github.io/polkadot-sdk/master/frame_support/pallet_macros/attr.hooks.html) to handle the snapshot event.
- event:
  `NextEpochMoved`
  #### Validator Election

- Top validators under `MaxValidators` and above `MinValidators` are selected based on the total amount of delegated amount and the total amount they bonded.
- If there is not enough validators (under the configured `MinValidators`), the active validator set is empty. By this way, there is no block produced and no reward distributed.
- In this pallet, the top validators will be sorted out and selected at the beginning of the new epoch.
  #### Rewards
- When starting epoch block, we calculate an amount of bond for rewarding. For simple, we choose a formula: 5% of total staking amount. And add this bond to reward storage of each delegator and validator.
- We have an event: `RewardClaimed`. When Delegator undelegate, we will trigger this event and deposit reward to delegator.

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
 ## References
  - Solo-chain-template: https://github.com/paritytech/polkadot-sdk-solochain-template/tree/master
  - Polkadot academy material: https://github.com/Polkadot-Blockchain-Academy/pba-content/tree/main/syllabus/6-Polkadot-SDK
  - Shout out Chase Chung for supporting me complete this course: https://github.com/chungquantin/substrate-dpos