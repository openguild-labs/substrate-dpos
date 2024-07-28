# Substrate Delegated Proof of Stake blockchain
Delegated Proof of Stake (DPoS) is a blockchain consensus mechanism where network users vote and elect delegates to validate the next block.

## Table of Contents
- [Introduction](#introduction)
- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [Configuration](#configuration)
- [Contributing](#contributing)
- [License](#license)
- [Acknowledgements](#acknowledgements)

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