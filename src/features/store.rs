use std::collections::BTreeMap;

use super::{
    account::{Account, ClientId},
    transaction::{Transaction, TransactionId},
};

/// This keeps track of users' account aggregation, deposits and withdrawals
#[derive(Debug, Clone)]
pub struct Store {
    pub client_accounts: BTreeMap<ClientId, Account>,
    pub transactions: BTreeMap<TransactionId, Transaction>,
}

impl Store {
    pub(crate) fn new() -> Self {
        Self {
            client_accounts: BTreeMap::new(),
            transactions: BTreeMap::new(),
        }
    }
}
