use std::collections::BTreeMap;

use super::{
    account::{Account, Client},
    transaction::{Transaction, TransactionId},
};

/// This keeps track of users' account aggregation, deposits and withdrawals
#[derive(Debug)]
pub struct Store {
    pub(crate) accounts: BTreeMap<Client, Account>,
    pub transactions: BTreeMap<TransactionId, Transaction>,
}

impl Store {
    pub(crate) fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
            transactions: BTreeMap::new(),
        }
    }
}
