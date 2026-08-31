use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use crate::wal::{WriteAheadLog, LogOp};

pub struct MemTable {
    pub map: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
    wal: WriteAheadLog,
    wal_path: PathBuf,
    pub size_bytes: usize,
}

impl MemTable {
    // Open a MemTable and recover state from WAL if it exists
    pub fn open<P: AsRef<Path>>(wal_path: P) -> io::Result<Self> {
        let wal_path_buf = wal_path.as_ref().to_path_buf();
        let mut map = BTreeMap::new();
        let mut size_bytes = 0;

        // Recover previous state if WAL file exists
        if wal_path_buf.exists() {
            let ops = WriteAheadLog::recover(&wal_path_buf)?;
            for op in ops {
                match op {
                    LogOp::Put(key, val) => {
                        size_bytes += key.len() + val.len();
                        map.insert(key, Some(val));
                    }
                    LogOp::Delete(key) => {
                        size_bytes += key.len();
                        map.insert(key, None); // Delete tombstone
                    }
                }
            }
        }

        // Open WAL for append-writing
        let wal = WriteAheadLog::open(&wal_path_buf)?;

        Ok(Self {
            map,
            wal,
            wal_path: wal_path_buf,
            size_bytes,
        })
    }

    pub fn put(&mut self, key: Vec<u8>, val: Vec<u8>) -> io::Result<()> {
        self.wal.append_put(&key, &val)?;
        self.size_bytes += key.len() + val.len();
        self.map.insert(key, Some(val));
        Ok(())
    }

    pub fn delete(&mut self, key: Vec<u8>) -> io::Result<()> {
        self.wal.append_delete(&key)?;
        self.size_bytes += key.len();
        self.map.insert(key, None);
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Option<&Option<Vec<u8>>> {
        self.map.get(key)
    }

    // Reset MemTable, clear WAL, and return sorted snapshot
    pub fn clear_and_reset(&mut self) -> io::Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>> {
        let snapshot = std::mem::take(&mut self.map);
        self.size_bytes = 0;

        // Truncate current WAL file
        if self.wal_path.exists() {
            fs::remove_file(&self.wal_path)?;
        }
        self.wal = WriteAheadLog::open(&self.wal_path)?;

        Ok(snapshot)
    }
}
