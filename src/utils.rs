use rand::Rng;
use tokio::signal;
use std::process::exit;
use primitive_types::U256;
use super::ascii_art::print_exit_art;

#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};

pub async fn handle_exit_signals() {
    #[cfg(unix)]
    {
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to create SIGTERM handler");
        tokio::select! {
            _ = sigterm.recv() => {}
            _ = signal::ctrl_c() => {}
        }
    }

    #[cfg(not(unix))]
    {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
    }

    print_exit_art();
    exit(0);
}

pub fn generate_nonce() -> String {
    let mut rng = rand::thread_rng();
    let nonce: u32 = rng.gen();
    format!("{:08x}", nonce)
}

pub fn meets_target(hash: &[u8], target: &[u8]) -> bool {
    // 使用 from_big_endian 来直接从字节切片创建 U256
    let target_int = U256::from_big_endian(target);
    let hash_int = U256::from_big_endian(hash);

    // 比较 U256 值
    hash_int < target_int
}