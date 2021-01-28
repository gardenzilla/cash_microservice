use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct TransactionLog {
  // Stored transactions
  transactions: Vec<Transaction>,
  // Balance
  balance_cash: i32,
}

impl Default for TransactionLog {
  fn default() -> Self {
    TransactionLog {
      transactions: Vec::new(),
      balance_cash: 0,
    }
  }
}

impl TransactionLog {
  /// Add Transaction to transactions
  pub fn add_transaction(&mut self, transaction: Transaction) -> Result<&Transaction, String> {
    match transaction.kind {
      // Incerement balance if cash
      TransactionKind::Cash => self.balance_cash += transaction.amount,
      _ => (),
    }
    // Store transaction
    self.transactions.push(transaction.clone());
    // Find the last transaction and return a ref of it
    if let Some(tr) = self.transactions.last() {
      if tr.id == transaction.id {
        return Ok(tr);
      }
    }
    Err(format!(
      "Error while inserting, or getting the inserted value"
    ))
  }
  /// Get current balance
  pub fn get_balance(&self) -> i32 {
    self.balance_cash
  }
  pub fn get_transactions(&self) -> &Vec<Transaction> {
    &self.transactions
  }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum TransactionKind {
  Cash,
  Card,
  Transfer,
}

impl Default for TransactionKind {
  fn default() -> Self {
    TransactionKind::Cash
  }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
  // Random transaction ID using UUID
  pub id: Uuid,
  // Optional cart_id, only
  // if the payment is related to a cart
  pub cart_id: Option<String>,
  // Transaction Kind
  pub kind: TransactionKind,
  // Payment
  pub amount: i32,
  // Reference
  pub reference: String,
  // Comment
  pub comment: String,
  // Created by UID
  pub created_by: u32,
  // Created at
  pub created_at: DateTime<Utc>,
}

impl Transaction {
  pub fn new(
    cart_id: Option<String>,
    kind: TransactionKind,
    amount: i32,
    reference: String,
    comment: String,
    created_by: u32,
  ) -> Self {
    Self {
      id: uuid::Uuid::new_v4(),
      cart_id,
      kind,
      amount,
      reference,
      comment,
      created_by,
      created_at: Utc::now(),
    }
  }
}

impl Default for Transaction {
  fn default() -> Self {
    Transaction {
      id: Uuid::default(),
      cart_id: None,
      kind: TransactionKind::default(),
      amount: 0,
      reference: String::default(),
      comment: String::default(),
      created_by: 0,
      created_at: Utc::now(),
    }
  }
}
