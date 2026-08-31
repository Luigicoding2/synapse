use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub mod wal;
pub mod sstable;
pub mod memtable;

use sstable::SSTable;
use memtable::MemTable;

#[derive(Debug)]
pub enum StoreError {
    IO(io::Error),
    Lock,
    Other(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::IO(err) => write!(f, "IO Error: {}", err),
            StoreError::Lock => write!(f, "Thread Lock Poisoned"),
            StoreError::Other(msg) => write!(f, "Store Error: {}", msg),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<io::Error> for StoreError {
    fn from(err: io::Error) -> Self {
        StoreError::IO(err)
    }
}

// 4MB Flush threshold
const FLUSH_THRESHOLD_BYTES: usize = 4 * 1024 * 1024;

pub struct SynapseStore {
    db_dir: PathBuf,
    memtable: Arc<RwLock<MemTable>>,
    sstables: Arc<RwLock<Vec<(u32, SSTable)>>>, // Sorted (Index, Reader) newest first
}

impl SynapseStore {
    // Open database directory and load previous WAL/SSTables
    pub fn open<P: AsRef<Path>>(dir: P) -> Result<Self, StoreError> {
        let db_dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&db_dir)?;

        // 1. Recover SSTables
        let mut sst_files = Vec::new();
        for entry in fs::read_dir(&db_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name().unwrap().to_string_lossy();
                if filename.starts_with("sst_") && filename.ends_with(".db") {
                    // Extract index number: e.g. sst_00004.db -> 4
                    let idx_str = &filename[4..filename.len() - 3];
                    if let Ok(idx) = idx_str.parse::<u32>() {
                        sst_files.push((idx, path));
                    }
                }
            }
        }

        // Sort files numerically, oldest to newest
        sst_files.sort_by_key(|(idx, _)| *idx);

        let mut sstables = Vec::new();
        for (idx, path) in sst_files {
            let table = SSTable::open(path)?;
            sstables.push((idx, table));
        }

        // Keep list order: newest first
        sstables.reverse();

        // 2. Open active MemTable / WAL
        let wal_path = db_dir.join("wal.log");
        let memtable = MemTable::open(wal_path)?;

        Ok(Self {
            db_dir,
            memtable: Arc::new(RwLock::new(memtable)),
            sstables: Arc::new(RwLock::new(sstables)),
        })
    }

    // Write a key-value pair
    pub fn put(&self, key: &[u8], val: &[u8]) -> Result<(), StoreError> {
        let mut mem = self.memtable.write().map_err(|_| StoreError::Lock)?;
        mem.put(key.to_vec(), val.to_vec())?;

        // Auto-flush if threshold is crossed
        if mem.size_bytes >= FLUSH_THRESHOLD_BYTES {
            drop(mem); // Release lock before flushing
            self.flush()?;
        }
        Ok(())
    }

    // Read value for a key
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StoreError> {
        // 1. Search in-memory MemTable
        {
            let mem = self.memtable.read().map_err(|_| StoreError::Lock)?;
            if let Some(val_opt) = mem.get(key) {
                return Ok(val_opt.clone());
            }
        }

        // 2. Search on-disk SSTables (newest to oldest)
        let mut ssts = self.sstables.write().map_err(|_| StoreError::Lock)?;
        for (_, sst) in ssts.iter_mut() {
            if let Some(val_opt) = sst.get(key)? {
                return Ok(val_opt); // Returns Some(val) or None (deleted)
            }
        }

        Ok(None)
    }

    // Mark key as deleted (tombstone)
    pub fn delete(&self, key: &[u8]) -> Result<(), StoreError> {
        let mut mem = self.memtable.write().map_err(|_| StoreError::Lock)?;
        mem.delete(key.to_vec())?;

        if mem.size_bytes >= FLUSH_THRESHOLD_BYTES {
            drop(mem);
            self.flush()?;
        }
        Ok(())
    }

    // Flush active MemTable to disk as an SSTable file
    pub fn flush(&self) -> Result<(), StoreError> {
        let mut mem = self.memtable.write().map_err(|_| StoreError::Lock)?;
        if mem.map.is_empty() {
            return Ok(());
        }

        // Reset MemTable state and extract snapshot
        let snapshot = mem.clear_and_reset()?;
        drop(mem); // Release MemTable write lock

        let mut ssts = self.sstables.write().map_err(|_| StoreError::Lock)?;
        
        // Find next SSTable index number
        let next_idx = ssts.iter().map(|(idx, _)| *idx).max().unwrap_or(0) + 1;
        let sst_filename = format!("sst_{:05}.db", next_idx);
        let sst_path = self.db_dir.join(&sst_filename);

        // Map BTreeMap to slice
        let sorted_data: Vec<(Vec<u8>, Option<Vec<u8>>)> = snapshot.into_iter().collect();
        SSTable::write_new(&sst_path, &sorted_data)?;

        // Open newly created reader and insert at index 0 (newest)
        let sst_reader = SSTable::open(&sst_path)?;
        ssts.insert(0, (next_idx, sst_reader));

        Ok(())
    }

    // Consolidates all active SSTables into a single file to improve search speeds
    pub fn compact(&self) -> Result<(), StoreError> {
        // Ensure no MemTable flushes occur during compaction
        let _mem_lock = self.memtable.write().map_err(|_| StoreError::Lock)?;
        
        let mut ssts = self.sstables.write().map_err(|_| StoreError::Lock)?;
        if ssts.len() <= 1 {
            return Ok(()); // Nothing to compact
        }

        // Merge-sort all SSTable keys from oldest to newest to preserve overrides
        let mut consolidated: BTreeMap<Vec<u8>, Option<Vec<u8>>> = BTreeMap::new();

        // Traverse from oldest to newest so newer keys overwrite older ones
        for (_, sst) in ssts.iter_mut().rev() {
            // Collect keys first to avoid simultaneous immutable+mutable borrow
            let keys: Vec<Vec<u8>> = sst.index.iter().map(|r| r.key.clone()).collect();
            for key in keys {
                if let Some(val_opt) = sst.get(&key)? {
                    consolidated.insert(key, val_opt);
                }
            }
        }

        // Clean up: Filter out explicit tombstones (None) during consolidation to free disk space!
        let compacted_data: Vec<(Vec<u8>, Option<Vec<u8>>)> = consolidated
            .into_iter()
            .filter(|(_, val_opt)| val_opt.is_some())
            .collect();

        // Write to a temporary file
        let temp_filename = "sst_compaction.tmp";
        let temp_path = self.db_dir.join(temp_filename);
        SSTable::write_new(&temp_path, &compacted_data)?;

        // Close all readers to release file locks on Windows
        let old_indices: Vec<u32> = ssts.iter().map(|(idx, _)| *idx).collect();
        ssts.clear();

        // Delete old sst files
        for idx in old_indices {
            let path = self.db_dir.join(format!("sst_{:05}.db", idx));
            if path.exists() {
                fs::remove_file(path)?;
            }
        }

        // Rename compacted file to sst_00001.db
        let final_path = self.db_dir.join("sst_00001.db");
        fs::rename(&temp_path, &final_path)?;

        // Open consolidated table
        let sst_reader = SSTable::open(&final_path)?;
        ssts.push((1, sst_reader));

        Ok(())
    }
}
