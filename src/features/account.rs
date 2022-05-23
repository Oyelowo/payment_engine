use super::store::Store;
use super::transaction::{Transaction, TransactionId};
use anyhow::Context;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize, Serializer};
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
    #[error("Action forbidden, account- (0) is locked")]
    AccountLocked(ClientId),

    #[error("Invalid input")]
    InvalidInput(#[from] anyhow::Error),

    #[error("Unknown")]
    Unknown,
}

type AccountResult<T> = anyhow::Result<T, AccountError>;

/// Client Account
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Account {
    client_id: ClientId,
    /// The total funds that are available for trading, staking, withdrawal, etc.
    /// This should be equal to the total - held amounts
    #[serde(rename = "available", serialize_with = "round_serialize")]
    available_amount: Decimal,

    /// The total funds that are held for dispute. This should be equal to total - available amounts
    #[serde(rename = "held", serialize_with = "round_serialize")]
    held_amount: Decimal,

    /// The total funds that are available or held. This should be equal to available + held
    #[serde(rename = "total", serialize_with = "round_serialize")]
    total_amount: Decimal,

    /// Whether the account is locked. An account is locked if a charge back occurs
    #[serde(rename = "locked")]
    is_locked: bool,
}

fn round_serialize<S>(amount: &Decimal, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Serialize to 4 decimal
    s.serialize_str(format!("{amount:.4}").as_str())
}

impl Account {
    pub(crate) fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            available_amount: dec!(0),
            held_amount: dec!(0),
            total_amount: dec!(0),
            is_locked: false,
        }
    }

    pub fn find_or_create_by_client_id(client_id: ClientId, store: &mut Store) -> Account {
        *store
            .client_accounts
            .entry(client_id)
            .or_insert_with(|| Account::new(client_id))
    }

    pub(crate) fn update(self, store: &mut Store) -> AccountResult<Self> {
        let account = Self::find_or_create_by_client_id(self.client_id, store);
        if account.is_locked {
            return Err(AccountError::AccountLocked(self.client_id));
        }

        store.client_accounts.insert(self.client_id, self);
        Ok(self)
    }

    pub(crate) fn deposit(self, amount: Decimal, store: &mut Store) -> AccountResult<Self> {
        Self {
            available_amount: self.available_amount + amount,
            total_amount: self.total_amount + amount,
            ..self
        }
        .update(store)
    }

    pub(crate) fn withdraw(self, amount: Decimal, store: &mut Store) -> AccountResult<Self> {
        if self.available_amount < amount {
            return Err(AccountError::InsufficientFund {
                requested: amount,
                available: self.available_amount,
            });
        }

        Self {
            available_amount: self.available_amount - amount,
            total_amount: self.total_amount - amount,
            ..self
        }
        .update(store)
    }

    pub(crate) fn dispute(
        self,
        transaction_id: TransactionId,
        store: &mut Store,
    ) -> AccountResult<Self> {
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
        Ok(self)
    }

    pub(crate) fn resolve(
        self,
        transaction_id: TransactionId,
        store: &mut Store,
    ) -> AccountResult<Self> {
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
        Ok(self)
    }

    // Should charge back be allowed to negative balance?
    pub(crate) fn charge_back(
        self,
        transaction_id: TransactionId,
        store: &mut Store,
    ) -> AccountResult<Self> {
        let mut transaction = Transaction::find_by_id(transaction_id, store);

        match transaction.as_mut() {
            Some(t) if t.is_under_dispute => {
                let amount = t.amount.context("Amount does not exist")?;
                Self {
                    is_locked: true,
                    held_amount: self.held_amount - amount,
                    total_amount: self.total_amount - amount,
                    ..self
                }
                .update(store)
            }
            _ => Err(AccountError::Unknown),
        }
    }
}
