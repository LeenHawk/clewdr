use std::sync::LazyLock;
use std::sync::{
    Mutex,
    atomic::{AtomicI64, AtomicU64, Ordering},
};

// metrics
pub static LAST_WRITE_TS: LazyLock<AtomicI64> = LazyLock::new(|| AtomicI64::new(0));
pub static WRITE_ERROR_COUNT: LazyLock<AtomicU64> = LazyLock::new(|| AtomicU64::new(0));
pub static TOTAL_WRITES: LazyLock<AtomicU64> = LazyLock::new(|| AtomicU64::new(0));
pub static TOTAL_WRITE_NANOS: LazyLock<AtomicU64> = LazyLock::new(|| AtomicU64::new(0));
pub static LAST_ERROR: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

// connection metrics
pub static LAST_CONN_TS: LazyLock<AtomicI64> = LazyLock::new(|| AtomicI64::new(0));
pub static CONN_SUCCESS_COUNT: LazyLock<AtomicU64> = LazyLock::new(|| AtomicU64::new(0));
pub static CONN_ERROR_COUNT: LazyLock<AtomicU64> = LazyLock::new(|| AtomicU64::new(0));

#[derive(Clone, Debug, Default)]
pub struct ConnConfigMetrics {
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_ms: u64,
    pub acquire_timeout_ms: u64,
    pub idle_timeout_ms: u64,
}

pub static CONN_CONFIG: LazyLock<Mutex<Option<ConnConfigMetrics>>> =
    LazyLock::new(|| Mutex::new(None));

// transaction metrics
pub static LAST_TX_TS: LazyLock<AtomicI64> = LazyLock::new(|| AtomicI64::new(0));
pub static TX_BEGIN_COUNT: LazyLock<AtomicU64> = LazyLock::new(|| AtomicU64::new(0));
pub static TX_COMMIT_COUNT: LazyLock<AtomicU64> = LazyLock::new(|| AtomicU64::new(0));
pub static TX_ROLLBACK_COUNT: LazyLock<AtomicU64> = LazyLock::new(|| AtomicU64::new(0));

pub fn mark_write_ok() {
    let now = chrono::Utc::now().timestamp();
    LAST_WRITE_TS.store(now, Ordering::Relaxed);
}
pub fn mark_write_err() {
    WRITE_ERROR_COUNT.fetch_add(1, Ordering::Relaxed);
}
pub fn record_duration(start: std::time::Instant) {
    let dur = start.elapsed();
    TOTAL_WRITES.fetch_add(1, Ordering::Relaxed);
    TOTAL_WRITE_NANOS.fetch_add(dur.as_nanos() as u64, Ordering::Relaxed);
}
pub fn record_error_msg(e: &dyn std::error::Error) {
    if let Ok(mut g) = LAST_ERROR.lock() {
        *g = Some(e.to_string());
    }
}

pub fn mark_connect_ok() {
    let now = chrono::Utc::now().timestamp();
    LAST_CONN_TS.store(now, Ordering::Relaxed);
    CONN_SUCCESS_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn mark_connect_err() {
    CONN_ERROR_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn set_conn_config(cfg: ConnConfigMetrics) {
    if let Ok(mut g) = CONN_CONFIG.lock() {
        *g = Some(cfg);
    }
}

pub fn mark_tx_begin() {
    TX_BEGIN_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn mark_tx_commit() {
    let now = chrono::Utc::now().timestamp();
    LAST_TX_TS.store(now, Ordering::Relaxed);
    TX_COMMIT_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn mark_tx_rollback() {
    TX_ROLLBACK_COUNT.fetch_add(1, Ordering::Relaxed);
}
