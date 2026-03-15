//! Cloud server state shared across Axum handlers.

use crate::db::CloudDb;
use crate::router::TenantRouter;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CloudState {
    pub db: CloudDb,
    pub router: Arc<RwLock<TenantRouter>>,
    pub jwt_secret: String,
}

impl CloudState {
    pub fn new(db: CloudDb, router: TenantRouter, jwt_secret: String) -> Self {
        Self {
            db,
            router: Arc::new(RwLock::new(router)),
            jwt_secret,
        }
    }
}
