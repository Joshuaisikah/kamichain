use kamichain_core::error::KamiError;
use kamichain_core::Chain;
use std::path::PathBuf;
pub struct Storage {
    path: PathBuf,
}
impl Storage {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Storage { path: path.into() }
    }
    pub fn save_chain(&self, chain: &Chain) -> Result<(), KamiError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| KamiError::InvalidChain(e.to_string()))?;
        }
        let tmp_path = self.path.with_extension("tmp");
        let json =
            serde_json::to_string(chain).map_err(|e| KamiError::InvalidChain(e.to_string()))?;
        std::fs::write(&tmp_path, json).map_err(|e| KamiError::InvalidChain(e.to_string()))?;
        std::fs::rename(&tmp_path, &self.path)
            .map_err(|e| KamiError::InvalidChain(e.to_string()))?;
        Ok(())
    }
    pub fn load_chain(&self) -> Result<Chain, KamiError> {
        let json = std::fs::read_to_string(&self.path)
            .map_err(|e| KamiError::InvalidChain(e.to_string()))?;
        let chain =
            serde_json::from_str(&json).map_err(|e| KamiError::InvalidChain(e.to_string()))?;
        Ok(chain)
    }
}
