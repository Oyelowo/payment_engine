use std::collections::BTreeMap;

use super::{
    account::{Account, Client},
    transaction::{Transaction, TransactionId},
};

/// This keeps track of users' account aggregation, deposits and withdrawals
#[derive(Debug, Default)]
pub struct Store {
    pub(crate) accounts: BTreeMap<Client, Account>,
    pub transactions: BTreeMap<TransactionId, Transaction>,
}