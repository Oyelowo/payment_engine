use super::store::Store;
use super::transaction::{Transaction, TransactionId};
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub(crate) type ClientId = u16;

#[derive(Error, Debug)]
pub enum AccountError {
    #[error(
        "You cannot withdraw {requested}. It is less than {available} available in your account"
    )]
    InsufficientFund {
        requested: Decimal,
        available: Decimal,
    },
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct ClientAccount {
    client_id: ClientId,
    /// The total funds that are available for trading, staking, withdrawal, etc.
    /// This should be equal to the total - held amounts
    /// Four decimal places
    #[serde(rename = "available", with = "rust_decimal::serde::float")]
    available_amount: Decimal,

    /// The total funds that are held for dispute. This should be equal to total - available amounts
    #[serde(rename = "held")]
    held_amount: Decimal,

    /// The total funds that are available or held. This should be equal to available + held
    #[serde(rename = "total")]
    total_amount: Decimal,

    /// Whether the account is locked. An account is locked if a charge back occurs
    #[serde(rename = "locked")]
    is_locked: bool,
}

impl ClientAccount {
    pub(crate) fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            available_amount: dec!(0.0000),
            held_amount: dec!(0.0000),
            total_amount: dec!(0.0000),
            is_locked: false,
        }
    }

    pub fn find_or_create_by_client_id(client_id: ClientId, store: &mut Store) -> ClientAccount {
        store
            .client_accounts
            .get(&client_id)
            // TODO: Check this
            .map_or(ClientAccount::new(client_id), |x| *x)
    }

    pub(crate) fn update(self, store: &mut Store) -> Self {
        store.client_accounts.insert(self.client_id, self);
        self
    }

    pub(crate) fn deposit(self, amount: Decimal, store: &mut Store) -> Self {
        Self {
            available_amount: self.available_amount + amount,
            total_amount: self.total_amount + amount,
            ..self
        }
        .update(store)
    }

    pub(crate) fn withdraw(
        self,
        amount: Decimal,
        store: &mut Store,
    ) -> anyhow::Result<Self, AccountError> {
        if self.available_amount < amount {
            return Err(AccountError::InsufficientFund {
                requested: amount,
                available: self.available_amount,
            });
        }

        Ok(Self {
            available_amount: self.available_amount - amount,
            total_amount: self.total_amount - amount,
            ..self
        }
        .update(store))
    }

    pub(crate) fn dispute(self, transaction_id: TransactionId, store: &mut Store) -> Self {
        let mut transaction = Transaction::find_by_id(transaction_id, store);

        if let Some(transaction) = transaction.as_mut() {
            transaction.is_under_dispute = true;
        }

        if let Some(amount) = transaction.and_then(|x| x.amount) {
            return Self {
                available_amount: self.available_amount - amount,
                held_amount: self.held_amount + amount,
                ..self
            }
            .update(store);
        }
        self
    }

    pub(crate) fn resolve(self, transaction_id: TransactionId, store: &mut Store) -> Self {
        let mut maybe_transaction = Transaction::find_by_id(transaction_id, store);

        if let Some(transaction) = maybe_transaction.as_mut() {
            transaction.is_under_dispute = false;
        }

        if let Some(amount) = maybe_transaction.and_then(|x| x.amount) {
            return Self {
                available_amount: self.available_amount + amount,
                held_amount: self.held_amount - amount,
                ..self
            }
            .update(store);
        }
        self
    }

    pub(crate) fn charge_back(self, transaction_id: TransactionId, store: &mut Store) -> Self {
        let mut transaction = Transaction::find_by_id(transaction_id, store);

        if let Some(transaction) = transaction.as_mut() {
            if transaction.is_under_dispute {
                if let Some(amount) = transaction.amount {
                    return Self {
                        is_locked: true,
                        held_amount: self.held_amount - amount,
                        total_amount: self.total_amount - amount,
                        ..self
                    }
                    .update(store);
                }
            }
        }

        self
    }
}
