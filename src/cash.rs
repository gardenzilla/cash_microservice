use chrono::{DateTime, Utc};
use packman::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct TransactionLog {
    next_id: u32,
    transactions: Vec<Transaction>,
    balance: i32,
}

impl Default for TransactionLog {
    fn default() -> Self {
        TransactionLog {
            next_id: 1,
            transactions: Vec::new(),
            balance: 0,
        }
    }
}

impl TransactionLog {
    pub fn add_transaction(
        &mut self,
        kind: &str,
        amount: i32,
        reference: String,
        created_by: String,
    ) -> Result<&Transaction, String> {
        let kind = TransactionKind::from_str(kind)?;
        let transaction: Transaction = Transaction {
            id: self.next_id,
            kind,
            amount,
            reference,
            created_by,
            date_created: Utc::now(),
        };
        // Incerement balance
        self.balance += transaction.amount;
        // Increment next id
        self.next_id += 1;
        self.transactions.push(transaction);
        match self.transactions.last() {
            Some(tr) => Ok(tr),
            None => Err(format!(
                "Error while inserting, or getting the inserted value"
            )),
        }
    }
    pub fn get_balance(&self) -> i32 {
        self.balance
    }
    pub fn get_log(&self, from: DateTime<Utc>, till: DateTime<Utc>) -> Vec<&Transaction> {
        self.transactions
            .iter()
            .filter(|t| t.date_created >= from && t.date_created <= till)
            .collect::<Vec<&Transaction>>()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub(crate) id: u32,
    pub(crate) kind: TransactionKind,
    pub(crate) amount: i32,
    pub(crate) reference: String,
    pub(crate) created_by: String,
    pub(crate) date_created: DateTime<Utc>,
}

impl Default for Transaction {
    fn default() -> Self {
        Transaction {
            id: 0,
            kind: TransactionKind::default(),
            amount: 0,
            reference: String::default(),
            created_by: String::default(),
            date_created: Utc::now(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum TransactionKind {
    Purchase,
    MoneyIn,
    MoneyOut,
}

impl std::fmt::Display for TransactionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionKind::Purchase => write!(f, "purchase"),
            TransactionKind::MoneyIn => write!(f, "money_in"),
            TransactionKind::MoneyOut => write!(f, "money_out"),
        }
    }
}

impl Default for TransactionKind {
    fn default() -> Self {
        TransactionKind::Purchase
    }
}

impl TransactionKind {
    fn from_str(str: &str) -> Result<TransactionKind, String> {
        use TransactionKind::*;
        match str {
            "purchase" => Ok(Purchase),
            "money_in" => Ok(MoneyIn),
            "money_out" => Ok(MoneyOut),
            _ => Err(format!(
                "A tranzakció kód nem megfelelő. Az alábbi lehet: {}, {}, {}",
                TransactionKind::Purchase,
                TransactionKind::MoneyIn,
                TransactionKind::MoneyOut
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add_transaction() {
        use TransactionKind::*;
        let mut tlog = TransactionLog::default();
        let a = tlog
            .add_transaction(&Purchase.to_string(), 100, "".into(), "demo".into())
            .unwrap()
            .clone();
        let b = tlog
            .add_transaction(&Purchase.to_string(), 200, "".into(), "demo".into())
            .unwrap()
            .clone();
        let c = tlog
            .add_transaction(&Purchase.to_string(), 300, "".into(), "demo".into())
            .unwrap()
            .clone();
        let d = tlog
            .add_transaction(&Purchase.to_string(), 400, "".into(), "demo".into())
            .unwrap()
            .clone();
        assert_eq!(a.id, 1);
        assert_eq!(b.id, 2);
        assert_eq!(c.id, 3);
        assert_eq!(d.id, 4);
        assert_eq!(a.amount, 100);
        assert_eq!(b.amount, 200);
        assert_eq!(c.amount, 300);
        assert_eq!(d.amount, 400);
        assert_eq!(tlog.get_balance(), 1000);
    }
}
