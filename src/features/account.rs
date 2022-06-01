use super::store::Store;
use super::transaction::{Transaction, TransactionId};
use anyhow::Context;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Client(u16);

#[derive(Error, Debug)]
pub(crate) enum AccountError {
    #[error(
        "You cannot withdraw {requested}. It is less than {available} available in your account"
    )]
    InsufficientFund {
        requested: Decimal,
        available: Decimal,
    },
    #[error("Action forbidden, account- (0) is locked")]
    AccountLocked(Client),

    #[error("Invalid input")]
    InvalidInput(#[from] anyhow::Error),

    #[error("Erroneous dispute: Transaction id (0)")]
    ErroneousDispute(TransactionId),

    #[error("Erroneous resolve: Transaction id (0)")]
    ErroneousResolve(TransactionId),

    #[error("Erroneous charge back: Transaction id (0)")]
    ErroneousChargeback(TransactionId),
}

type AccountResult<T> = anyhow::Result<T, AccountError>;

/// Client Account
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Account {
    client: Client,
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
    let rounded_amount = amount.round_dp(4).to_string();
    s.serialize_str(rounded_amount.as_str())
}

impl Account {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            available_amount: dec!(0),
            held_amount: dec!(0),
            total_amount: dec!(0),
            is_locked: false,
        }
    }

    pub(crate) fn find_or_create_by_client(client: Client, store: &mut Store) -> Account {
        *store
            .accounts
            .entry(client)
            .or_insert_with(|| Account::new(client))
    }

    pub(crate) fn update(self, store: &mut Store) -> AccountResult<Self> {
        let account = Self::find_or_create_by_client(self.client, store);
        if account.is_locked {
            return Err(AccountError::AccountLocked(self.client));
        }

        store.accounts.insert(self.client, self);
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
        let existing_transaction = Transaction::find_by_id(transaction_id, store);
        match existing_transaction {
            Some(tx) => {
                let amount = tx.get_amount().with_context(|| "Amount does not exist")?;
                tx.set_is_under_dispute(true);

                Self {
                    available_amount: self.available_amount - amount,
                    held_amount: self.held_amount + amount,
                    ..self
                }
                .update(store)
            }
            _ => Err(AccountError::ErroneousDispute(transaction_id)),
        }
    }

    pub(crate) fn resolve(
        self,
        transaction_id: TransactionId,
        store: &mut Store,
    ) -> AccountResult<Self> {
        let transaction = Transaction::find_by_id(transaction_id, store);
        match transaction {
            Some(tx) if tx.get_is_under_dispute() => {
                let amount = tx.get_amount().with_context(|| "Amount does not exist")?;
                tx.set_is_under_dispute(false);

                Self {
                    available_amount: self.available_amount + amount,
                    held_amount: self.held_amount - amount,
                    ..self
                }
                .update(store)
            }
            _ => Err(AccountError::ErroneousResolve(transaction_id)),
        }
    }

    // Should charge back be allowed to negative balance?
    pub(crate) fn charge_back(
        self,
        transaction_id: TransactionId,
        store: &mut Store,
    ) -> AccountResult<Self> {
        let existing_transaction = Transaction::find_by_id(transaction_id, store);

        match existing_transaction {
            Some(tx) if tx.get_is_under_dispute() => {
                let amount = tx.get_amount().with_context(|| "Amount does not exist")?;
                Self {
                    is_locked: true,
                    held_amount: self.held_amount - amount,
                    total_amount: self.total_amount - amount,
                    ..self
                }
                .update(store)
            }
            _ => Err(AccountError::ErroneousChargeback(transaction_id)),
        }
    }
}
