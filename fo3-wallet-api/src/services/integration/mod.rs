//! Service Integration Module
//! 
//! Handles cross-service communication, transaction coordination, and data consistency

pub mod service_coordinator;
pub mod transaction_manager;
pub mod event_dispatcher;
pub mod health_monitor;

pub use service_coordinator::ServiceCoordinator;
pub use transaction_manager::{TransactionManager, TransactionContext};
pub use event_dispatcher::{EventDispatcher, ServiceEvent};
pub use health_monitor::HealthMonitor;
