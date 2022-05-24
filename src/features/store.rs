use std::collections::BTreeMap;

use super::{
    account::{Account, ClientId},
    transaction::{Transaction, TransactionId},
};

/// This keeps track of users' account aggregation, deposits and withdrawals
#[derive(Debug, Clone)]
pub struct AccountStore {
    pub accounts: BTreeMap<ClientId, Account>,
}
impl AccountStore {
    pub(crate) fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
        }
    }
}

/// This keeps track of users' account aggregation, deposits and withdrawals
#[derive(Debug)]
pub struct TransactionStore {
    pub transactions: BTreeMap<TransactionId, Transaction>,
}

impl TransactionStore {
    pub(crate) fn new() -> Self {
        Self {
            transactions: BTreeMap::new(),
        }
    }
}
