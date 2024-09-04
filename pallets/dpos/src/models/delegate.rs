use crate::{BalanceOf, Config};

#[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
#[scale_info(skip_type_params(T))]
pub struct Delegation<T: Config> {
    pub amount: BalanceOf<T>,
}

impl<T: Config> Delegation<T> {
    pub fn new(amount: BalanceOf<T>) -> Self {
        Self { amount }
    }

    pub fn set_amount(&mut self, amount: BalanceOf<T>) {
        self.amount = amount;
    }
}