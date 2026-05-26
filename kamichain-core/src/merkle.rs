use sha2::{Digest, Sha256};
#[derive(Debug, Clone)]
pub struct  MarkleTree{
    leaves: Vec<String>,
    root: String,
}
impl MarkleTree{
    pub fn new(hashes: Vec<String>)->Self{
    if hashes.is_empty() {
        let root = hash_str("");
        return MarkleTree { leaves: vec![], root }
    }
        let leaves = hashes.clone();
        let mut level= hashes.iter().map(|h| hash_str(h)).collect::<Vec<_>>();
        while level.len() > 1 {
            let mut next_level = Vec::new();
            let mut i = 0;
            while i < level.len() {
                let left = &level[i];
                let right = if i + 1 < level.len() {
                    &level[i + 1]
                } else {
                    &level[i]
                };
                next_level.push(hash_pair(left, right));
                i += 2;
            }
            level = next_level;
        }
        MarkleTree {
                leaves,
                root:level[0].clone(),
            }
        }
    pub fn root(&self)->String{
        self.root.clone()
    }
    pub fn verify(&self,tx_hash:&str)->bool{
        self.leaves.contains(&tx_hash.to_string())
    }
}
fn hash_str(s: &str) -> String{
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}
fn hash_pair(left: &str, right: &str)->String{
    let mut hasher = Sha256::new();
    hasher.update(left.as_bytes());
    hasher.update(right.as_bytes());
    format!("{:x}", hasher.finalize())
}
