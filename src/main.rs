//           ,____           \'/
//       .-'` .   |        -= * =-
//     .'  '    ./           /.\
//    /  '    .'
//   ;  '    /
//  :  '  _ ;
// ;  :  /(\ \
// |  .       '.
// |  ' /     --'
// |  .   '.__\
// ;  :       /
//  ;  .     |            ,
//   ;  .    \           /|
//    \  .    '.       .'/
//     '.  '  . `'---'`.'
//       `'-..._____.-`
//
// Care about the emission. It’s freedom in code.
// Just a pulse in the network, a chance to be heard.
//
mod api;
mod ascii_art;
mod hasher;
mod models;
mod utils;
mod vail;

use crate::api::MinerState;
use ascii_art::*;
use colored::*;
use futures_util::{SinkExt, StreamExt};
use hasher::*;
use hex::FromHex;
use models::*;
use primitive_types::U256;
use rand::Rng;
use std::sync::Arc;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use sha2::{Digest, Sha256};

use std::time::Duration;

use std::{path, thread};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use utils::*;
use vail::*;
extern crate core_affinity;


#[tokio::main]
async fn main() {
    let args = Args::parse_and_validate();
    std::panic::set_hook(Box::new(|_info| {}));

    let max_workers = num_cpus::get();
    assert!(max_workers > 0);

    let num_workers = match args.threads {
        Some(t) => {
            if t >= max_workers {
                println!(
                    "{} (max workers: {})",
                    "Requested number of threads exceeds available cores. Using maximum allowed"
                        .bold()
                        .red(),
                    max_workers
                );
                max_workers
            } else {
                t
            }
        }
        None => max_workers,
    };
    let num_reversetimes = match args.reversetimes {
        Some(t) => t,
        None => 1000,  // Default value = 1000
    };



    println!("{}", "STARTING MINER".bold().green());
    println!(
        "{} {}",
        "USING WORKERS: ".bold().cyan(),
        format!("{}", num_workers).bold().cyan()
    );
    print_startup_art();

    tokio::spawn(handle_exit_signals());

    // let bailout_timer: u64 = match args.vdftime_parsed {
    //     Some(timer) => timer,
    //     None => 20,
    // };
    let miner_id = args.address.unwrap();

    let (server_sender, server_receiver) = mpsc::channel::<String>();

    let current_job: Arc<Mutex<Option<Job>>> = Arc::new(Mutex::new(None));

    let miner_state = Arc::new(MinerState {
        hash_count: Arc::new(AtomicUsize::new(0)),
        accepted_shares: Arc::new(AtomicUsize::new(0)),
        rejected_shares: Arc::new(AtomicUsize::new(0)),
        hashrate_samples: Arc::new(Mutex::new(Vec::new())),
        version: String::from("1.0.0"),
    });
    let core_ids = core_affinity::get_core_ids().unwrap();
    let hash_count = Arc::new(AtomicUsize::new(0));
    let accepted_shares = Arc::new(AtomicUsize::new(0));
    let handles: Vec<_> = core_ids
        .into_iter()
        .enumerate()
        .filter_map(|(i, id)| {
            if i < num_workers {
                let current_job_loop = Arc::clone(&current_job);
                let hash_count = Arc::clone(&hash_count);
                let accepted_shares = Arc::clone(&accepted_shares);
                let server_sender_clone = server_sender.clone();
                let mut miner_id = miner_id.clone();
                let api_hash_count = Arc::clone(&miner_state.hash_count);
                // Create a new thread
                let builder = thread::Builder::new().stack_size(64 * 1024 * 1024);
                Some(
                    builder
                        .spawn(move || {
                            let res = core_affinity::set_for_current(id);
                            if res {
                                loop {
                                    let job_option = {
                                        let job_guard = current_job_loop.blocking_lock();
                                        job_guard.clone()
                                    };
                                    if let Some(job) = job_option {
                                        'newjob: loop {
                                            let (nonce, /*hash_first,*/ result,path_all,ifpath) =
                                                generate_nonce_and_find_cycle(&job.target, &job.data,7000, num_reversetimes);
                                            // Get a path
                                            if ifpath {
                                                hash_count.fetch_add(num_reversetimes, Ordering::Relaxed);
                                                api_hash_count.fetch_add(num_reversetimes, Ordering::Relaxed);
                                            } else {
                                                continue;
                                            }
                                            if !path_all.is_empty() { // Seems a bug, but work now.
                                                    for (nonce_hex, path_hex) in result.clone() {
                                                        let submit_msg = SubmitMessage {
                                                            r#type: String::from("submit"),
                                                            miner_id: miner_id.to_string(),
                                                            nonce: nonce_hex.clone(),
                                                            job_id: job.job_id.clone(),
                                                            path: path_hex.clone(),
                                                        };
                                                        println!(
                                                            "{}",
                                                            format!("Submit Nonce = {}", nonce_hex)
                                                                .bold()
                                                                .purple()
                                                        );
                                                        accepted_shares
                                                            .fetch_add(1, Ordering::Relaxed);
                                                        let msg: String =
                                                            serde_json::to_string(&submit_msg)
                                                                .unwrap();
                                                        let _ = server_sender_clone.send(msg);
                                                    }    
                                                let new_job_option = {
                                                    let job_guard = current_job_loop.blocking_lock();
                                                    job_guard.clone()
                                                };
                                                if new_job_option.is_none()
                                                    || new_job_option.unwrap().job_id != job.job_id
                                                {
                                                    break 'newjob;
                                                }                                            
                                            }
                                        }
                                    }
                                }
                            }
                        })
                        .unwrap(),
                )
            } else {
                None
            }
        })
        .collect();

    tokio::spawn(async move {
        let mut last_count = 0;
        let mut time = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let count = hash_count.load(Ordering::Relaxed);
            let share_count = accepted_shares.load(Ordering::Relaxed);
            time = time + 5;
            println!(
                "{}: {} KH/s",
                "Hash Rate/second".blue(),
                (count - last_count) / 5_000
            );
            println!("{}: {} ", "total shares: ".cyan(), share_count);
            last_count = count;
        }
    });

    let api_hashrate_clone = miner_state.clone();
    tokio::spawn(async move {
        let mut last_count = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await; // Measure every second
            let current_count = api_hashrate_clone.hash_count.load(Ordering::Relaxed);
            let hashes_per_second = current_count - last_count;
            let mut samples = api_hashrate_clone.hashrate_samples.lock().await;
            samples.push(hashes_per_second as u64);
            if samples.len() > 10 {
                samples.remove(0);
            }
            last_count = current_count;
        }
    });

    let api_state = miner_state.clone();
    tokio::spawn(api::start_http_server(api_state));

    let current_job_clone = Arc::clone(&current_job);
    let request_clone = args.pool.unwrap().clone();

    let server_receiver = Arc::new(Mutex::new(server_receiver));

    loop {
        let request = request_clone.clone().into_client_request().unwrap();
        let (ws_stream, _) = match connect_async(request).await {
            Ok((ws_stream, response)) => (ws_stream, response),
            Err(_e) => {
                let delay_secs = rand::thread_rng().gen_range(5..30);
                println!(
                    "{}",
                    format!("Failed to connect will retry in {} seconds...", delay_secs).red()
                );
                tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                continue;
            }
        };

        let (write, mut read) = ws_stream.split();

        let server_receiver_clone = Arc::clone(&server_receiver);
        tokio::spawn(async move {
            let mut write = write;
            while let Ok(msg) = {
                let receiver = server_receiver_clone.lock().await;
                receiver.recv()
            } {
                write.send(Message::Text(msg)).await.unwrap();
            }
        });

        loop {
            match read.next().await {
                Some(Ok(msg)) => match msg {
                    Message::Text(text_msg) => {
                        let server_message: ServerMessage =
                            serde_json::from_str(&text_msg).unwrap();
                        match server_message.r#type.as_str() {
                            "job" => {
                                if let (Some(job_id), Some(data), Some(target)) = (
                                    server_message.job_id.clone(),
                                    server_message.data.clone(),
                                    server_message.target.clone(),
                                ) {
                                    let mut data_array = [0u8; 76];
                                    hex::decode_to_slice(data.clone(), &mut data_array).unwrap();
                                    let mut target_array = [0u8; 32];
                                    hex::decode_to_slice(target.clone(), &mut target_array)
                                        .unwrap();
                                    let new_job = Job {
                                        job_id: job_id.clone(),
                                        data: data_array,
                                        target: target_array,
                                    };
                                    // println!("data size = {}", new_job.data.len());
                                    // println!("target size = {}", new_job.target.len());

                                    let mut job_guard = current_job_clone.lock().await;
                                    *job_guard = Some(new_job);

                                    println!(
                                        "{} {}",
                                        "Received new job:".bold().blue(),
                                        format!(
                                            "ID = {}, Data = {}, Target = {}",
                                            job_id, data, target
                                        )
                                        .bold()
                                        .yellow()
                                    );
                                }
                            }
                            "accepted" => {
                                miner_state.accepted_shares.fetch_add(1, Ordering::Relaxed);
                                println!("{}", format!("Share accepted").bold().green());
                                //display_share_accepted();
                            }
                            "rejected" => {
                                miner_state.rejected_shares.fetch_add(1, Ordering::Relaxed);
                                println!("{}", "Share rejected.".red());
                            }
                            _ => {}
                        }
                    }
                    Message::Close(_) => {
                        println!("{}", "You are now a frog.".green());
                        std::process::exit(0);
                    }
                    _ => {}
                },
                Some(Err(_e)) => {
                    println!(
                        "{}",
                        "WebSocket connection closed. Will sleep then try to reconnect.".red()
                    );
                    break;
                }
                None => {
                    println!(
                        "{}",
                        "WebSocket connection closed. Will sleep then try to reconnect.".red()
                    );
                    break;
                }
            }
        }

        let mut job_guard = current_job_clone.lock().await;
        *job_guard = None;

        let delay_secs = rand::thread_rng().gen_range(11..42);
        println!(
            "{}",
            format!("Reconnecting in {} seconds...", delay_secs).yellow()
        );
        tokio::time::sleep(Duration::from_secs(delay_secs)).await;
        println!("{}", "Attempting to reconnect...".red());
    }
}
