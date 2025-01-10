use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use rand::Rng;
use tokio::signal;
use tokio_icmp_echo::Pinger;

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
    // Send an ICMP echo request and wait for a reply
    // eprintln!("pinging: {}", host);

    match pinger.ping(host.parse().ok()?, 0xabcd, 0, Duration::from_millis(250)).await {
        Ok(reply) => {
            // eprintln!("Pinged: {}", host);
            // note : on timeout we get None here
           reply
        }
        Err(_) => {
            eprintln!("Failed to ping host: {}", host);
            None
        }
    }
}

async fn monitor_host(host: String, stats: Arc<Mutex<HashMap<String, LatencyStats>>>) {
    let pinger = Pinger::new().await.unwrap();
    eprintln!("starting: {}", host);

    // Wait a random amount of time between 0-500ms before pinging
    let delay = rand::thread_rng().gen_range(0..500);
    sleep(Duration::from_millis(delay)).await;

    loop {
       
         // Ping the host and measure latency
         if let Some(latency) = ping_host(&pinger, &host).await {
            // Update the latency statistics
            let mut stats = stats.lock().await;
            let entry = stats.entry(host.clone()).or_insert_with(LatencyStats::new);
            
            entry.update(latency.as_millis());

            println!(
                "Host: {}, Latency: {}ms, Avg Latency: {:.2}ms",
                host, latency.as_millis(), entry.average_latency
            );
        }

        // Wait 500ms before the next iteration
        sleep(Duration::from_millis(500)).await;
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let hosts = vec!["20.236.44.162", "142.250.75.142", "13.226.2.72"];
    // let hosts = vec!["8.8.8.8"];
    let stats = Arc::new(Mutex::new(HashMap::<String, LatencyStats>::new()));

    // Spawn a task for each host to monitor its latency
    for host in hosts {
        let stats = Arc::clone(&stats);
        tokio::spawn(monitor_host(host.to_string(), stats));
    }

    // Wait for Ctrl-C or SIGTERM to exit
    signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");
    println!("Exiting...");
}