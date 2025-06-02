//! Health service implementation

use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;

use crate::proto::fo3::wallet::v1::{
    health_service_server::HealthService,
    *,
};

pub struct HealthServiceImpl;

impl HealthServiceImpl {
    pub fn new() -> Self {
        Self
    }
}

#[tonic::async_trait]
impl HealthService for HealthServiceImpl {
    async fn check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let response = HealthCheckResponse {
            status: health_check_response::ServingStatus::Serving as i32,
        };

        Ok(Response::new(response))
    }

    type WatchStream = ReceiverStream<Result<HealthCheckResponse, Status>>;

    async fn watch(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(4);

        tokio::spawn(async move {
            loop {
                let response = HealthCheckResponse {
                    status: health_check_response::ServingStatus::Serving as i32,
                };

                if tx.send(Ok(response)).await.is_err() {
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
