use super::store::Store;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ClientId(u16);

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
    AccountLocked(ClientId),

    #[error("Invalid input")]
    InvalidInput(#[from] anyhow::Error),
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

    pub(crate) fn find_or_create_by_client_id(client_id: ClientId, store: &mut Store) -> Account {
        *store
            .accounts
            .entry(client_id)
            .or_insert_with(|| Account::new(client_id))
    }

    pub(crate) fn update(self, store: &mut Store) -> AccountResult<Self> {
        let existing_account = Self::find_or_create_by_client_id(self.client_id, store);
        if existing_account.is_locked {
            return Err(AccountError::AccountLocked(self.client_id));
        }

        store.accounts.insert(self.client_id, self);
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

    pub(crate) fn dispute(self, amount: Decimal, store: &mut Store) -> AccountResult<Self> {
        // let existing_account = Account::find_or_create_by_client_id(self.client_id, store);
        Self {
            available_amount: self.available_amount - amount,
            held_amount: self.held_amount + amount,
            ..self
        }
        .update(store)
    }

    pub(crate) fn resolve(self, amount: Decimal, store: &mut Store) -> AccountResult<Self> {
        // let existing_account = Account::find_or_create_by_client_id(self.client_id, store);

        Self {
            available_amount: self.available_amount + amount,
            held_amount: self.held_amount - amount,
            ..self
        }
        .update(store)
    }

    // Should charge back be allowed to negative balance?
    pub(crate) fn charge_back(self, amount: Decimal, store: &mut Store) -> AccountResult<Self> {
        Self {
            is_locked: true,
            held_amount: self.held_amount - amount,
            total_amount: self.total_amount - amount,
            ..self
        }
        .update(store)
    }
}
