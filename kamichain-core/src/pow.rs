use rayon::prelude::*;
use sha2::{Digest, Sha256};
use crate::block::Block;
use crate::error::KamiError;

/// Number of nonces each thread batch searches before the outer loop advances.
const CHUNK: u64 = 100_000;

pub struct ProofOfWork {
    pub difficulty: usize,
}

impl ProofOfWork {
    pub fn new(difficulty: usize) -> Self {
        ProofOfWork { difficulty }
    }

    pub fn target_prefix(&self) -> String {
        "0".repeat(self.difficulty)
    }

    /// Mine a valid nonce for `block` using all available CPU cores.
    ///
    /// The nonce space (u64) is split into chunks of CHUNK values.
    /// Each chunk is searched in parallel with Rayon — `find_any` stops
    /// all threads in the chunk the moment one finds a valid hash.
    /// The outer loop advances to the next chunk only if the current one
    /// yields nothing.
    pub fn mine(&self, block: &mut Block) {
        let target    = self.target_prefix();
        let index     = block.index;
        let timestamp = block.timestamp;
        let merkle    = block.merkle_root.clone();
        let prev      = block.prev_hash.clone();

        let nonce = (0u64..)
            .step_by(CHUNK as usize)
            .find_map(|start| {
                (start..start.saturating_add(CHUNK))
                    .into_par_iter()
                    .find_any(|&n| {
                        hash_candidate(index, timestamp, &merkle, &prev, n)
                            .starts_with(&target)
                    })
            })
            .expect("nonce space exhausted");

        block.nonce = nonce;
        block.hash  = block.compute_hash();
    }

    pub fn validate(&self, block: &Block) -> Result<(), KamiError> {
        if block.hash != block.compute_hash() {
            return Err(KamiError::InvalidPoW);
        }
        if !block.hash.starts_with(&self.target_prefix()) {
            return Err(KamiError::InvalidPoW);
        }
        Ok(())
    }
}

/// Compute a candidate block hash for the given fields and nonce.
/// Mirrors `Block::compute_hash` exactly — same field order, same SHA-256 call.
fn hash_candidate(index: u64, timestamp: u64, merkle: &str, prev: &str, nonce: u64) -> String {
    let input = format!("{}{}{}{}{}", index, timestamp, merkle, prev, nonce);
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    format!("{:x}", h.finalize())
}
