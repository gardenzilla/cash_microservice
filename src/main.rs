extern crate gzlib;

mod cash;
use gzlib::proto::cash::cash_service_client as client;
use gzlib::proto::cash::cash_service_server as server;
// use gzlib::proto::user::*;
use packman::*;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use tokio::sync::{oneshot, Mutex};
use tonic::transport::Channel;
use tonic::{transport::Server, Request, Response, Status};

use gzlib::proto;

struct CashService {
    // Day (Date)
    transactions: Mutex<Pack<cash::TransactionLog>>,
}

impl CashService {
    fn init(transactions: Pack<cash::TransactionLog>) -> CashService {
        CashService {
            transactions: Mutex::new(transactions),
        }
    }
}

#[tonic::async_trait]
impl server::CashService for CashService {
    async fn transaction(
        &self,
        request: Request<proto::cash::TransactionRequest>,
    ) -> Result<Response<proto::cash::TransactionResponse>, Status> {
        let r = request.into_inner();
        let tr = &mut self.transactions.lock().await;
        let mut _tr = tr.as_mut();
        let res = _tr
            .unpack()
            .add_transaction(&r.kind, r.amount, r.reference, r.created_by)
            .map_err(|e| Status::internal(e))?;
        Ok(Response::new(proto::cash::TransactionResponse {
            transaction_id: res.id,
            amount: res.amount,
            kind: res.kind.to_string(),
            reference: res.reference.to_string(),
            created_at: res.date_created.to_rfc3339(),
            created_by: res.created_by.to_string(),
        }))
    }

    async fn get_balance(
        &self,
        request: Request<proto::cash::BalanceRequest>,
    ) -> Result<Response<proto::cash::Balance>, Status> {
        let balance = self.transactions.lock().await.unpack().get_balance();
        Ok(Response::new(proto::cash::Balance { balance }))
    }

    async fn log(
        &self,
        request: Request<proto::cash::LogRequest>,
    ) -> Result<Response<proto::cash::LogResponse>, Status> {
        use chrono::{DateTime, Utc};
        use proto::cash::TransactionResponse;
        let r = request.into_inner();
        let from = chrono::DateTime::parse_from_rfc3339(&r.from)
            .map_err(|_| Status::invalid_argument("From date invalid"))?;
        let till = chrono::DateTime::parse_from_rfc3339(&r.till)
            .map_err(|_| Status::invalid_argument("Till date invalid"))?;
        let res = self
            .transactions
            .lock()
            .await
            .unpack()
            .get_log(DateTime::<Utc>::from(from), DateTime::<Utc>::from(till))
            .iter()
            .map(|i| TransactionResponse {
                transaction_id: i.id,
                kind: i.kind.to_string(),
                amount: i.amount,
                reference: i.reference.to_string(),
                created_by: i.created_by.to_string(),
                created_at: i.date_created.to_rfc3339(),
            })
            .collect::<Vec<TransactionResponse>>();
        Ok(Response::new(proto::cash::LogResponse {
            transaction_details: res,
        }))
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
        .unwrap_or("[::1]:50051".into())
        .parse()
        .unwrap();

    // Create shutdown channel
    let (tx, rx) = oneshot::channel();

    // Spawn the server into a runtime
    tokio::task::spawn(async move {
        Server::builder()
            .add_service(server::CashServiceServer::new(CashService::init(
                transactions,
            )))
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
