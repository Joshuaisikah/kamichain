use crate::error::NodeError;

#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub bind_addr:  String,
    pub rpc_addr:   String,
    pub difficulty: usize,
    pub data_dir:   String,
    pub miner_addr: String,
    pub peer:       Option<String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            bind_addr:   "127.0.0.1:8333".to_string(),
            rpc_addr:    "127.0.0.1:8332".to_string(),
            difficulty:  2,
            data_dir:    "./data".to_string(),
            miner_addr:  "default_miner".to_string(),
            peer:        None,
        }
    }
}

impl NodeConfig {
    pub fn from_args() -> Result<Self, NodeError> {
        let args: Vec<String> = std::env::args().collect();
        Self::from_slice(&args)
    }

    // takes a slice so tests can pass args without touching process::args
    pub fn from_slice(args: &[String]) -> Result<Self, NodeError> {
        let mut cfg = NodeConfig::default();

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--bind" => {
                    cfg.bind_addr = next_value(args, i, "--bind")?;
                    i += 2;
                }
                "--rpc" => {
                    cfg.rpc_addr = next_value(args, i, "--rpc")?;
                    i += 2;
                }
                "--difficulty" => {
                    let raw = next_value(args, i, "--difficulty")?;
                    cfg.difficulty = raw.parse().map_err(|_| {
                        NodeError::Config(format!("invalid difficulty: {}", raw))
                    })?;
                    i += 2;
                }
                "--data-dir" => {
                    cfg.data_dir = next_value(args, i, "--data-dir")?;
                    i += 2;
                }
                "--miner" => {
                    cfg.miner_addr = next_value(args, i, "--miner")?;
                    i += 2;
                }
                "--peer" => {
                    cfg.peer = Some(next_value(args, i, "--peer")?);
                    i += 2;
                }
                flag => {
                    return Err(NodeError::Config(format!("unknown flag: {}", flag)));
                }
            }
        }

        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<(), NodeError> {
        if self.difficulty == 0 {
            return Err(NodeError::Config("difficulty must be >= 1".to_string()));
        }
        if self.miner_addr.is_empty() {
            return Err(NodeError::Config("miner address must not be empty".to_string()));
        }
        Ok(())
    }

    pub fn chain_path(&self) -> String {
        format!("{}/chain.json", self.data_dir)
    }
}

fn next_value(args: &[String], i: usize, flag: &str) -> Result<String, NodeError> {
    args.get(i + 1)
        .cloned()
        .ok_or_else(|| NodeError::Config(format!("{} requires a value", flag)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(pairs: &[&str]) -> Vec<String> {
        std::iter::once("node")
            .chain(pairs.iter().copied())
            .map(String::from)
            .collect()
    }

    #[test]
    fn defaults_when_no_args() {
        let cfg = NodeConfig::from_slice(&args(&[])).unwrap();
        assert_eq!(cfg.bind_addr, "127.0.0.1:8333");
        assert_eq!(cfg.difficulty, 2);
        assert!(cfg.peer.is_none());
    }

    #[test]
    fn parses_all_flags() {
        let cfg = NodeConfig::from_slice(&args(&[
            "--bind", "0.0.0.0:9000",
            "--rpc",  "0.0.0.0:9001",
            "--difficulty", "4",
            "--data-dir", "/tmp/kami",
            "--miner", "alice",
            "--peer", "192.168.1.2:8333",
        ])).unwrap();

        assert_eq!(cfg.bind_addr,  "0.0.0.0:9000");
        assert_eq!(cfg.rpc_addr,   "0.0.0.0:9001");
        assert_eq!(cfg.difficulty, 4);
        assert_eq!(cfg.data_dir,   "/tmp/kami");
        assert_eq!(cfg.miner_addr, "alice");
        assert_eq!(cfg.peer,       Some("192.168.1.2:8333".to_string()));
    }

    #[test]
    fn rejects_zero_difficulty() {
        let err = NodeConfig::from_slice(&args(&["--difficulty", "0"])).unwrap_err();
        assert!(err.to_string().contains("difficulty"));
    }

    #[test]
    fn rejects_unknown_flag() {
        let err = NodeConfig::from_slice(&args(&["--unknown", "x"])).unwrap_err();
        assert!(err.to_string().contains("unknown flag"));
    }

    #[test]
    fn rejects_flag_without_value() {
        let err = NodeConfig::from_slice(&args(&["--bind"])).unwrap_err();
        assert!(err.to_string().contains("requires a value"));
    }

    #[test]
    fn chain_path_uses_data_dir() {
        let cfg = NodeConfig::from_slice(&args(&["--data-dir", "/var/kami"])).unwrap();
        assert_eq!(cfg.chain_path(), "/var/kami/chain.json");
    }
}
