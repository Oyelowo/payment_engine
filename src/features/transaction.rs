use super::account::{Account, AccountError, ClientId};
use super::store::Store;
use anyhow::Context;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TransactionType {
    ///  A deposit is a credit to the client's asset account, meaning it should increase the available and total funds of the client account
    Deposit,

    /// A withdraw is a debit to the client's asset account, meaning it should decrease the available and total funds of the client account

    /// If a client does not have sufficient available funds the withdrawal should fail and the total amount
    /// of funds should not change
    Withdrawal,

    /// A dispute represents a client's claim that a transaction was erroneous and should be reversed.
    /// The transaction shouldn't be reversed yet but the associated funds should be held. This means
    /// that the clients available funds should decrease by the amount disputed, their held funds should
    /// increase by the amount disputed, while their total funds should remain the same.

    /// Notice that a dispute does not state the amount disputed. Instead a dispute references the
    /// transaction that is disputed by ID. If the tx specified by the dispute doesn't exist you can ignore it
    /// and assume this is an error on our partners side.
    Dispute,

    /// A resolve represents a resolution to a dispute, releasing the associated held funds. Funds that
    /// were previously disputed are no longer disputed. This means that the clients held funds should
    /// decrease by the amount no longer disputed, their available funds should increase by the
    /// amount no longer disputed, and their total funds should remain the same.
    Resolve,

    /// A chargeback is the final state of a dispute and represents the client reversing a transaction.
    /// Funds that were held have now been withdrawn. This means that the clients held funds and
    /// total funds should decrease by the amount previously disputed. If a chargeback occurs the
    /// client's account should be immediately frozen.

    /// Like a dispute and a resolve a chargeback refers to the transaction by ID (tx) and does not
    /// specify an amount. Like a resolve, if the tx specified doesn't exist, or the tx isn't under dispute,
    /// you can ignore chargeback and assume this is an error on our partner's side.
    Chargeback,
}

#[derive(Error, Debug)]
pub(crate) enum TransactionError {
    #[error("Invalid transaction - {0}")]
    AccountError(#[from] AccountError),

    #[error("Invalid input - {0}")]
    InvalidAmount(Decimal),

    #[error("Unknown")]
    Unknown(#[from] anyhow::Error),
}

type TransactionResult<T> = anyhow::Result<T, TransactionError>;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct TransactionId(u32);

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Transaction {
    #[serde(rename = "type")]
    transaction_type: TransactionType,

    /// Unique but not guaranteed to be ordered
    #[serde(rename = "client")]
    client_id: ClientId,

    /// Globally Unique but not guaranteed to be ordered
    #[serde(rename = "tx")]
    transaction_id: TransactionId,

    /// Four decimal places
    amount: Option<Decimal>,

    #[serde(skip)]
    is_under_dispute: bool,
}

impl Transaction {
    pub fn find_by_id(
        transaction_id: TransactionId,
        store: &mut Store,
    ) -> Option<&mut Transaction> {
        store.transactions.get_mut(&transaction_id)
    }

    pub(crate) fn save(self, store: &mut Store) -> TransactionResult<()> {
        use TransactionType::*;

        if let Some(amount) = self.amount {
            if amount < dec!(0) {
                return Err(TransactionError::InvalidAmount(amount));
            }
        }

        if let Deposit | Withdrawal = self.transaction_type {
            store.transactions.insert(self.transaction_id, self);
        }

        self.update_account(store)?;

        Ok(())
    }

    fn update_account(self, store: &mut Store) -> TransactionResult<()> {
        use TransactionType::*;

        let existing_account = Account::find_or_create_by_client_id(self.client_id, store);
        let amount = self.amount.with_context(|| "Unable to get amount");

        match self.transaction_type {
            Deposit => existing_account.deposit(amount?, store)?,
            Withdrawal => existing_account.withdraw(amount?, store)?,
            Dispute => existing_account.dispute(self.transaction_id, store)?,
            Resolve => existing_account.resolve(self.transaction_id, store)?,
            Chargeback => existing_account.charge_back(self.transaction_id, store)?,
        };
        Ok(())
    }

    /// Get the transaction's is under dispute.
    pub fn get_is_under_dispute(&self) -> bool {
        self.is_under_dispute
    }

    /// Get the transaction's amount.
    pub fn get_amount(&self) -> Option<Decimal> {
        self.amount
    }

    /// Set the transaction's is under dispute.
    pub fn set_is_under_dispute(&mut self, is_under_dispute: bool) {
        self.is_under_dispute = is_under_dispute;
    }
}
