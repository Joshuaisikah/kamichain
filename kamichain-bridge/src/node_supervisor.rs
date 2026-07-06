use std::collections::VecDeque;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{broadcast, mpsc, Mutex};

const RECENT_LOG_CAPACITY: usize = 300;

#[derive(Clone)]
pub struct NodeConfig {
    pub node_bin_path: String,
    pub rpc_addr: String,
    pub bind_addr: String,
    pub difficulty: String,
    pub data_dir: String,
    pub miner_address: String,
}

pub struct LogHub {
    pub tx: broadcast::Sender<String>,
    pub recent: Mutex<VecDeque<String>>,
}

impl LogHub {
    pub fn new() -> Arc<Self> {
        let (tx, _rx) = broadcast::channel(256);
        Arc::new(LogHub {
            tx,
            recent: Mutex::new(VecDeque::with_capacity(RECENT_LOG_CAPACITY)),
        })
    }

    async fn push(&self, line: String) {
        let mut recent = self.recent.lock().await;
        if recent.len() >= RECENT_LOG_CAPACITY {
            recent.pop_front();
        }
        recent.push_back(line.clone());
        drop(recent);
        // A send error just means no SSE clients are currently connected.
        let _ = self.tx.send(line);
    }

    pub async fn recent_lines(&self) -> Vec<String> {
        self.recent.lock().await.iter().cloned().collect()
    }
}

/// Owns the kamichain-node child process for the lifetime of the bridge.
/// Wipes the data dir and (re)spawns on startup, on an unexpected exit, and
/// whenever a reset is requested over `reset_rx` — this is what makes the
/// public demo instance safely resettable.
pub async fn run(cfg: NodeConfig, log: Arc<LogHub>, mut reset_rx: mpsc::Receiver<()>) {
    loop {
        log.push(format!("[bridge] starting kamichain-node (difficulty={})", cfg.difficulty))
            .await;

        if let Err(e) = std::fs::remove_dir_all(&cfg.data_dir) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log.push(format!("[bridge] warning: failed to wipe data dir: {}", e)).await;
            }
        }

        let mut child = match Command::new(&cfg.node_bin_path)
            .args([
                "--rpc",
                &cfg.rpc_addr,
                "--bind",
                &cfg.bind_addr,
                "--difficulty",
                &cfg.difficulty,
                "--data-dir",
                &cfg.data_dir,
                "--miner",
                &cfg.miner_address,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                log.push(format!("[bridge] failed to spawn kamichain-node: {}", e)).await;
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let out_log = Arc::clone(&log);
        let err_log = Arc::clone(&log);

        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                out_log.push(line).await;
            }
        });
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                err_log.push(format!("[stderr] {}", line)).await;
            }
        });

        tokio::select! {
            status = child.wait() => {
                log.push(format!("[bridge] kamichain-node exited unexpectedly: {:?} — restarting", status)).await;
            }
            _ = reset_rx.recv() => {
                log.push("[bridge] reset requested — restarting with a fresh chain".to_string()).await;
                let _ = child.kill().await;
            }
        }
    }
}
