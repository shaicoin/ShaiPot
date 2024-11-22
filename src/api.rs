use warp::Filter;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Mutex;
use serde::Serialize;
use std::time::{Instant, Duration};

#[derive(Serialize)]
pub struct Stats {
    pub hashrate: u64,
    pub accepted: usize,
    pub rejected: usize,
    pub version: String,
    pub uptime: u64,
}

pub struct MinerState {
    pub hash_count: Arc<AtomicUsize>,
    pub accepted_shares: Arc<AtomicUsize>,
    pub rejected_shares: Arc<AtomicUsize>,
    pub hashrate_samples: Arc<Mutex<Vec<u64>>>,
    pub version: String,
}

async fn calculate_avg_hashrate(samples: Arc<Mutex<Vec<u64>>>) -> u64 {
    let samples = samples.lock().await;
    if samples.is_empty() {
        return 0;
    }
    let sum: u64 = samples.iter().sum();
    sum / samples.len() as u64
}

fn calculate_uptime(start_time: Instant) -> u64 {
    let elapsed: Duration = start_time.elapsed();
    elapsed.as_secs()
}

async fn stats_handler(state: Arc<MinerState>, start_time: Instant) -> Result<impl warp::Reply, warp::Rejection> {
    let avg_hashrate = calculate_avg_hashrate(state.hashrate_samples.clone()).await;
    let accepted = state.accepted_shares.load(Ordering::Relaxed);
    let rejected = state.rejected_shares.load(Ordering::Relaxed);
    let version = state.version.clone();
    let uptime = calculate_uptime(start_time);

    let stats = Stats {
        hashrate: avg_hashrate,
        accepted,
        rejected,
        version,
        uptime,
    };

    Ok(warp::reply::json(&stats))
}

pub async fn start_http_server(state: Arc<MinerState>) {
    let start_time = Instant::now();

    let stats_route = warp::path("stats")
        .and(warp::get())
        .and_then({
            let state = state.clone();
            move || stats_handler(state.clone(), start_time)
        });

    warp::serve(stats_route).run(([127, 0, 0, 1], 8844)).await;
}
