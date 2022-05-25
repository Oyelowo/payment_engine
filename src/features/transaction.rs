use super::account::{Account, AccountError, ClientId};
use super::store::Store;
use anyhow::Context;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("Invalid transaction - {0}")]
    AccountError(#[from] AccountError),

    #[error("Invalid input - {0}")]
    InvalidAmount(Decimal),

    #[error("Invalid input")]
    Unknown(#[from] anyhow::Error),
}

pub type TransactionId = u32;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct TransactionDetails {
    /// Unique but not guaranteed to be ordered
    #[serde(rename = "client")]
    client_id: ClientId,

    /// Globally Unique but not guaranteed to be ordered
    #[serde(rename = "tx")]
    transaction_id: TransactionId,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Transaction {
    ///  A deposit is a credit to the client's asset account, meaning it should increase the available and total funds of the client account
    Deposit {
        #[serde(flatten)]
        details: TransactionDetails,
        amount: Decimal,

        #[serde(skip)]
        is_under_dispute: bool,
    },

    /// A withdraw is a debit to the client's asset account, meaning it should decrease the available and total funds of the client account

    /// If a client does not have sufficient available funds the withdrawal should fail and the total amount
    /// of funds should not change
    Withdrawal {
        #[serde(flatten)]
        details: TransactionDetails,
        amount: Decimal,

        #[serde(skip)]
        is_under_dispute: bool,
    },

    /// A dispute represents a client's claim that a transaction was erroneous and should be reversed.
    /// The transaction shouldn't be reversed yet but the associated funds should be held. This means
    /// that the clients available funds should decrease by the amount disputed, their held funds should
    /// increase by the amount disputed, while their total funds should remain the same.

    /// Notice that a dispute does not state the amount disputed. Instead a dispute references the
    /// transaction that is disputed by ID. If the tx specified by the dispute doesn't exist you can ignore it
    /// and assume this is an error on our partners side.
    Dispute {
        #[serde(flatten)]
        details: TransactionDetails,
    },

    /// A resolve represents a resolution to a dispute, releasing the associated held funds. Funds that
    /// were previously disputed are no longer disputed. This means that the clients held funds should
    /// decrease by the amount no longer disputed, their available funds should increase by the
    /// amount no longer disputed, and their total funds should remain the same.
    Resolve {
        #[serde(flatten)]
        details: TransactionDetails,
    },

    /// A chargeback is the final state of a dispute and represents the client reversing a transaction.
    /// Funds that were held have now been withdrawn. This means that the clients held funds and
    /// total funds should decrease by the amount previously disputed. If a chargeback occurs the
    /// client's account should be immediately frozen.

    /// Like a dispute and a resolve a chargeback refers to the transaction by ID (tx) and does not
    /// specify an amount. Like a resolve, if the tx specified doesn't exist, or the tx isn't under dispute,
    /// you can ignore chargeback and assume this is an error on our partner's side.
    Chargeback {
        #[serde(flatten)]
        details: TransactionDetails,
    },
}

impl Transaction {
    pub fn find_by_id(
        transaction_id: TransactionId,
        store: &mut Store,
    ) -> Option<&mut Transaction> {
        store.transactions.get_mut(&transaction_id)
    }

    pub(crate) fn save(self, store: &mut Store) -> anyhow::Result<(), TransactionError> {
        if let Some(amount) = self.get_amount() {
            if amount < dec!(0) {
                return Err(TransactionError::InvalidAmount(amount));
            }
        }

        self.update_account(store)?;

        Ok(())
    }

    fn update_account(self, store: &mut Store) -> anyhow::Result<(), TransactionError> {
        use Transaction::*;

        // let existing_account = Account::find_or_create_by_client_id(self.client_id, store);
        // let amount = self.amount.with_context(|| "Unable to get amount");

        match self {
            Deposit {
                details, amount, ..
            } => {
                store.transactions.insert(details.transaction_id, self);
                let existing_account =
                    Account::find_or_create_by_client_id(details.client_id, store);
                existing_account.deposit(amount).update(store)?
            }
            Withdrawal {
                details, amount, ..
            } => {
                store.transactions.insert(details.transaction_id, self);
                let existing_account =
                    Account::find_or_create_by_client_id(details.client_id, store);
                existing_account.withdraw(amount, store)?
            }
            Dispute { details } => {
                let existing_account =
                    Account::find_or_create_by_client_id(details.client_id, store);
                let disputed_tx =
                    Self::find_by_id(details.transaction_id, store).context("not found")?;
                disputed_tx.set_is_under_dispute(true);
                let disputed_tx_amt = disputed_tx.get_amount().context("amount empty")?;

                existing_account.dispute(disputed_tx_amt).update(store)?
            }
            Resolve { details } => {
                let existing_account =
                    Account::find_or_create_by_client_id(details.client_id, store);
                let disputed_tx =
                    Self::find_by_id(details.transaction_id, store).context("not found")?;
                disputed_tx.set_is_under_dispute(false);
                let disputed_tx_amt = disputed_tx.get_amount().context("amount empty")?;
                existing_account.resolve(disputed_tx_amt).update(store)?
            }

            Chargeback { details } => {
                let existing_account =
                    Account::find_or_create_by_client_id(details.client_id, store);

                let disputed_tx =
                    Self::find_by_id(details.transaction_id, store).context("not found")?;
                // disputed_tx.set_is_under_dispute(true);
                let disputed_tx_amt = disputed_tx.get_amount().context("amount empty")?;
                existing_account
                    .charge_back(disputed_tx_amt)
                    .update(store)?
            }
        };
        Ok(())
    }

    /// Get the transaction's is under dispute.
    #[must_use]
    pub fn get_is_under_dispute(&self) -> bool {
        use Transaction::*;

        match self {
            Deposit {
                is_under_dispute, ..
            } => *is_under_dispute,
            Withdrawal {
                is_under_dispute, ..
            } => *is_under_dispute,
            _ => false,
        }

        // if let Chargeback {details} | Deposit { details, amount } = self  {
        //     details.is_under_dispute
        // }
        // self.is_under_dispute
    }

    /// Get the transaction's amount.
    #[must_use]
    pub fn get_amount(&self) -> Option<Decimal> {
        use Transaction::*;

        match self {
            Deposit { amount, .. } => Some(*amount),
            Withdrawal { amount: amount, .. } => Some(*amount),
            _ => None,
        }
    }

    /// Set the transaction's is under dispute.
    pub fn set_is_under_dispute(&mut self, is_under_dispute: bool) {
        use Transaction::*;

        match self {
            Deposit {
                is_under_dispute: k,
                ..
            } => {
                *k = is_under_dispute;
            }
            Withdrawal {
                is_under_dispute: k,
                ..
            } => {
                *k = is_under_dispute;
            }
            _ => {}
        }
    }
}
