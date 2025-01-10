use std::collections::HashMap;
use std::time::Duration;
use tokio::time::{sleep, Instant};
use rand::Rng;
use tokio::signal;
use tokio_icmp_echo::Pinger;
use futures::StreamExt;
use futures::stream::{FuturesUnordered};
use futures::future::BoxFuture;

type PingResult = (String, Pinger);

#[derive(Debug)]
struct LatencyStats {
    total_latency: u128,
    samples: u32,
    average_latency: f64,
}

impl LatencyStats {
    fn new() -> Self {
        LatencyStats {
            total_latency: 0,
            samples: 0,
            average_latency: 0.0,
        }
    }

    fn update(&mut self, latency: u128) {
        self.total_latency += latency;
        self.samples += 1;
        self.average_latency = self.total_latency as f64 / self.samples as f64;
    }
}

async fn ping_host(pinger: &Pinger, host: &str) -> Option<Duration> {
    match pinger.ping(host.parse().ok()?, 0xabcd, 0, Duration::from_millis(250)).await {
        Ok(reply) => reply,
        Err(_) => {
            eprintln!("Failed to ping host: {}", host);
            None
        }
    }
}

fn create_ping_future(host: String, pinger: Pinger, delay: u64) -> BoxFuture<'static, PingResult> {
    Box::pin(async move {
        sleep(Duration::from_millis(delay)).await;
        (host, pinger)
    })
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let hosts = vec!["20.236.44.162", "142.250.75.142", "13.226.2.72"];
    let mut stats = HashMap::<String, LatencyStats>::new();
    let mut futures: FuturesUnordered<BoxFuture<'static, PingResult>> = FuturesUnordered::new();

    // Initialize stats and create initial futures
    for host in hosts {
        stats.insert(host.to_string(), LatencyStats::new());
        
        // Random initial delay
        let delay = rand::thread_rng().gen_range(0..500);
        let pinger = Pinger::new().await.unwrap();
        let host = host.to_string();
        
        futures.push(create_ping_future(host, pinger, delay));
    }

    loop {
        tokio::select! {
            Some((host, pinger)) = futures.next() => {
                // Start the ping
                if let Some(latency) = ping_host(&pinger, &host).await {
                    // Update stats
                    if let Some(stats_entry) = stats.get_mut(&host) {
                        stats_entry.update(latency.as_millis());
                        
                        println!(
                            "Host: {}, Latency: {}ms, Avg Latency: {:.2}ms",
                            host, latency.as_millis(), stats_entry.average_latency
                        );
                    }
                }

                // Schedule next ping for this host
                futures.push(create_ping_future(host, pinger, 500));
            }
            
            _ = signal::ctrl_c() => {
                println!("Exiting...");
                break;
            }
        }
    }
}