use sp_std::collections::btree_map::BTreeMap;

#[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
#[scale_info(skip_type_params(T))]
pub struct Epoch<T: Config> {
	pub validators: BTreeMap<T::AccountId, BalanceOf<T>>,
	pub delegations: BTreeMap<(T::AccountId, T::AccountId), BalanceOf<T>>,
}

impl<T: Config> Epoch<T> {
	pub fn default() -> Self {
		Self { validators: BTreeMap::default(), delegations: BTreeMap::default() }
	}

	pub fn add_delegator(
		self: &mut Self,
		delegator: T::AccountId,
		candidate: T::AccountId,
		amount: BalanceOf<T>,
	) {
		self.delegations.insert((delegator, candidate), amount);
	}

	pub fn add_validator(self: &mut Self, candidate: T::AccountId, amount: BalanceOf<T>) {
		self.validators.insert(candidate, amount);
	}
}