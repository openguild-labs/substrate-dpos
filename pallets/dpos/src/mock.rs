use crate::{self as pallet_dpos, ReportNewValidatorSet};
use frame_support::{
	derive_impl, parameter_types,
	traits::{ConstU128, ConstU16, ConstU32, ConstU64, FindAuthor},
};
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;
type AccountId = u64;

// Configure a mock runtime to test the pallet. We use the simpler syntax here.
frame_support::construct_runtime! {
	pub struct Test {
		System: frame_system,
		Balances: pallet_balances,
		Dpos: pallet_dpos,
	}
}

// Feel free to remove more items from this, as they are the same as
// `frame_system::config_preludes::TestDefaultConfig`. We have only listed the full `type` list here
// for verbosity. Same for `pallet_balances::Config`.
// https://paritytech.github.io/polkadot-sdk/master/frame_support/attr.derive_impl.html
#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ConstU32<10>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<10>;
}

parameter_types! {
	pub const MaxValidators: u32 = 10;
}

pub struct AlwaysSeven;
impl FindAuthor<AccountId> for AlwaysSeven {
	fn find_author<'a, I>(_: I) -> Option<AccountId>
	where
		I: 'a + IntoIterator<Item = ([u8; 4], &'a [u8])>,
	{
		Some(7)
	}
}

pub struct DoNothing;
impl ReportNewValidatorSet<AccountId> for DoNothing {
	fn report_new_validator_set(_: Vec<AccountId>) {}
}

impl pallet_dpos::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type NativeBalance = Balances;
	type MaxValidators = MaxValidators;
	type FindAuthor = AlwaysSeven;
	type ReportNewValidatorSet = DoNothing;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	// learn how to improve your test setup:
	// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}