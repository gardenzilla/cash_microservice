mod cash;
mod prelude;

use cash::Transaction;
use chrono::{DateTime, Utc};
use gzlib::proto::cash::{cash_server::*, BalanceObject, LogRequest, NewTransaction};
use packman::*;
use prelude::*;
use proto::cash::TransactionObject;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use tokio::sync::{oneshot, Mutex};
use tonic::{transport::Server, Request, Response, Status};

use gzlib::proto;

struct CashService {
  transactions: Mutex<Pack<cash::TransactionLog>>,
}

impl CashService {
  // Init new service
  fn init(transactions: Pack<cash::TransactionLog>) -> CashService {
    CashService {
      transactions: Mutex::new(transactions),
    }
  }
  async fn create_transaction(&self, r: NewTransaction) -> ServiceResult<TransactionObject> {
    // Create new transaction object
    let new_transaction = Transaction::new(
      match r.cart_id {
        Some(cid) => match cid {
          proto::cash::new_transaction::CartId::Cart(id) => Some(id),
          proto::cash::new_transaction::CartId::None(_) => None,
        },
        None => None,
      },
      r.amount,
      r.reference,
      r.comment,
      r.created_by,
    );
    // Store new transaction in storage
    self
      .transactions
      .lock()
      .await
      .as_mut()
      .unpack()
      .add_transaction(new_transaction.clone())
      .map_err(|e| ServiceError::bad_request(&e))?;
    // Return transction as TransactionObject
    Ok(new_transaction.into())
  }
  // Get balance
  async fn get_balance(&self) -> ServiceResult<i32> {
    let res = self.transactions.lock().await.unpack().get_balance();
    Ok(res)
  }
  // Get transaction log
  async fn transaction_log(&self, r: LogRequest) -> ServiceResult<Vec<TransactionObject>> {
    // Define from date
    let from = DateTime::parse_from_rfc3339(&r.from)
      .map_err(|_| ServiceError::bad_request("A megadott -tól- dátum hibás"))?
      .with_timezone(&Utc);
    // Define till date
    let till = DateTime::parse_from_rfc3339(&r.till)
      .map_err(|_| ServiceError::bad_request("A megadott -ig- dátum hibás"))?
      .with_timezone(&Utc);
    // Filter transactions by dates
    let res = self
      .transactions
      .lock()
      .await
      .unpack()
      .get_transactions()
      .iter()
      .filter(|t| t.created_at >= from && t.created_at <= till)
      .map(|t| t.clone().into())
      .collect::<Vec<TransactionObject>>();
    // Return transactions as Vec<TransactionObject>
    Ok(res)
  }
}

#[tonic::async_trait]
impl Cash for CashService {
  async fn create_transaction(
    &self,
    request: Request<proto::cash::NewTransaction>,
  ) -> Result<Response<TransactionObject>, Status> {
    let res = self.create_transaction(request.into_inner()).await?;
    Ok(Response::new(res))
  }

  async fn get_balance(
    &self,
    _: Request<()>,
  ) -> Result<Response<proto::cash::BalanceObject>, Status> {
    let res = self.get_balance().await?;
    Ok(Response::new(BalanceObject { balance: res }))
  }

  type TransactionLogStream = tokio::sync::mpsc::Receiver<Result<TransactionObject, Status>>;

  async fn transaction_log(
    &self,
    request: Request<proto::cash::LogRequest>,
  ) -> Result<Response<Self::TransactionLogStream>, Status> {
    // Create channels
    let (mut tx, rx) = tokio::sync::mpsc::channel(100);
    // Get found price objects
    let res = self.transaction_log(request.into_inner()).await?;
    // Send found price_objects through the channel
    for transaction in res.into_iter() {
      tx.send(Ok(transaction))
        .await
        .map_err(|_| Status::internal("Error while sending price bulk over channel"))?
    }
    return Ok(Response::new(rx));
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  // Init database
  let transactions: Pack<cash::TransactionLog> =
    Pack::load_or_init(PathBuf::from("data"), "transactions")
      .expect("Error while loading transaction db :O");

  // Address is valid?
  // Todo: Implement auto address from environment variable
  let addr = env::var("SERVICE_ADDR_CASH")
    .unwrap_or("[::1]:50056".into())
    .parse()
    .unwrap();

  // Create shutdown channel
  let (tx, rx) = oneshot::channel();

  // Spawn the server into a runtime
  tokio::task::spawn(async move {
    Server::builder()
      .add_service(CashServer::new(CashService::init(transactions)))
      .serve_with_shutdown(addr, async {
        let _ = rx.await;
      })
      .await
      .unwrap()
  });

  tokio::signal::ctrl_c().await?;

  println!("SIGINT");

  // Send shutdown signal after SIGINT received
  let _ = tx.send(());

  Ok(())
}
