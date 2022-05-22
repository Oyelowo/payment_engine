use std::collections::HashMap;

use super::{
    account::{ClientAccount, ClientId},
    transaction::{Transaction, TransactionId},
};

#[derive(Debug, Clone)]
pub struct Store {
    pub client_accounts: HashMap<ClientId, ClientAccount>,
    pub transactions: HashMap<TransactionId, Transaction>,
}

impl Store {
    pub(crate) fn new() -> Self {
        Self {
            client_accounts: HashMap::new(),
            transactions: HashMap::new(),
        }
    }
}
