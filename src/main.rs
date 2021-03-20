mod cash;
mod prelude;

use cash::Transaction;
use chrono::{DateTime, Utc};
use gzlib::proto::cash::{
  cash_server::*, BalanceObject, BulkRequest, ByIdRequest, DateRangeRequest, NewTransaction,
  TransactionIds,
};
use packman::*;
use prelude::*;
use proto::cash::TransactionObject;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use tokio::sync::{oneshot, Mutex};
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;

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
    let tr: proto::cash::TransactionKind = proto::cash::TransactionKind::from_i32(r.kind).ok_or(
      ServiceError::internal_error("A tranzakció kód kódolásánál hiba történt"),
    )?;
    // Create new transaction object
    let new_transaction = Transaction::new(
      match r.cart_id {
        Some(cid) => match cid {
          proto::cash::new_transaction::CartId::Cart(id) => Some(id),
          proto::cash::new_transaction::CartId::None(_) => None,
        },
        None => None,
      },
      match tr {
        proto::cash::TransactionKind::KindCash => cash::TransactionKind::Cash,
        proto::cash::TransactionKind::KindCard => cash::TransactionKind::Card,
        proto::cash::TransactionKind::KindTransfer => cash::TransactionKind::Transfer,
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
  // Get by ID
  async fn get_by_id(&self, r: ByIdRequest) -> ServiceResult<TransactionObject> {
    // Transform id as String to UUID
    let id = Uuid::parse_str(&r.transaction_id)
      .map_err(|_| ServiceError::bad_request("Hibás tranzakció azonosító"))?;
    // Try find transaction object
    let res: TransactionObject = self
      .transactions
      .lock()
      .await
      .unpack()
      .get_transactions()
      .iter()
      .find(|tr| tr.id == id)
      .ok_or(ServiceError::not_found("A kért tranzakció nem található"))?
      .clone()
      .into();
    Ok(res)
  }
  // Get bulk
  async fn get_bulk(&self, r: BulkRequest) -> ServiceResult<Vec<TransactionObject>> {
    let res = self
      .transactions
      .lock()
      .await
      .unpack()
      .get_transactions()
      .iter()
      .filter(|tr| r.transaction_ids.contains(&tr.id.to_string()))
      .map(|tr| tr.clone().into())
      .collect::<Vec<TransactionObject>>();
    Ok(res)
  }
  // Get transaction log
  async fn get_by_date_range(&self, r: DateRangeRequest) -> ServiceResult<Vec<String>> {
    // Define from date
    let from = DateTime::parse_from_rfc3339(&r.date_from)
      .map_err(|_| ServiceError::bad_request("A megadott -tól- dátum hibás"))?
      .with_timezone(&Utc);
    // Define till date
    let till = DateTime::parse_from_rfc3339(&r.date_till)
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
      .map(|t| t.id.to_string())
      .collect::<Vec<String>>();
    // Return transactions as Vec<TransactionObject>
    Ok(res)
  }
}

#[tonic::async_trait]
impl Cash for CashService {
  async fn create_transaction(
    &self,
    request: Request<NewTransaction>,
  ) -> Result<Response<TransactionObject>, Status> {
    let res = self.create_transaction(request.into_inner()).await?;
    Ok(Response::new(res))
  }

  async fn get_balance(&self, _: Request<()>) -> Result<Response<BalanceObject>, Status> {
    let res = self.get_balance().await?;
    Ok(Response::new(BalanceObject { balance: res }))
  }

  async fn get_by_id(
    &self,
    request: Request<ByIdRequest>,
  ) -> Result<Response<TransactionObject>, Status> {
    let res = self.get_by_id(request.into_inner()).await?;
    Ok(Response::new(res))
  }

  type GetBulkStream = tokio::sync::mpsc::Receiver<Result<TransactionObject, Status>>;

  async fn get_bulk(
    &self,
    request: Request<BulkRequest>,
  ) -> Result<Response<Self::GetBulkStream>, Status> {
    // Create channels
    let (mut tx, rx) = tokio::sync::mpsc::channel(100);
    // Get found price objects
    let res = self.get_bulk(request.into_inner()).await?;
    // Send found price_objects through the channel
    tokio::spawn(async move {
      for transaction in res.into_iter() {
        tx.send(Ok(transaction)).await.unwrap();
      }
    });
    return Ok(Response::new(rx));
  }

  async fn get_by_date_range(
    &self,
    request: Request<DateRangeRequest>,
  ) -> Result<Response<TransactionIds>, Status> {
    let transaction_ids = self.get_by_date_range(request.into_inner()).await?;
    Ok(Response::new(TransactionIds { transaction_ids }))
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
