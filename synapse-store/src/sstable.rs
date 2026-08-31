use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct IndexRecord {
    pub key: Vec<u8>,
    pub offset: u64,
}

pub struct SSTable {
    file: File,
    pub index: Vec<IndexRecord>,
}

impl SSTable {
    // Write sorted data to a new SSTable file
    pub fn write_new<P: AsRef<Path>>(path: P, data: &[(Vec<u8>, Option<Vec<u8>>)]) -> io::Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        
        let mut writer = io::BufWriter::new(file);
        let mut index = Vec::new();
        let mut current_offset = 0u64;

        // 1. Write data block
        for (key, val_opt) in data {
            index.push(IndexRecord {
                key: key.clone(),
                offset: current_offset,
            });

            // Write: [Key Len: 4] [Val Len: 4 (Tombstone = 0xFFFFFFFF)] [Key] [Val]
            writer.write_all(&(key.len() as u32).to_le_bytes())?;
            current_offset += 4;

            match val_opt {
                Some(val) => {
                    writer.write_all(&(val.len() as u32).to_le_bytes())?;
                    current_offset += 4;
                    writer.write_all(key)?;
                    current_offset += key.len() as u64;
                    writer.write_all(val)?;
                    current_offset += val.len() as u64;
                }
                None => {
                    // 0xFFFFFFFF indicates tombstone
                    writer.write_all(&u32::MAX.to_le_bytes())?;
                    current_offset += 4;
                    writer.write_all(key)?;
                    current_offset += key.len() as u64;
                }
            }
        }

        // 2. Write index block
        let index_block_offset = current_offset;
        for record in &index {
            // Write: [Key Len: 4] [Offset: 8] [Key]
            writer.write_all(&(record.key.len() as u32).to_le_bytes())?;
            writer.write_all(&record.offset.to_le_bytes())?;
            writer.write_all(&record.key)?;
        }

        // 3. Write footer: index_block_offset (8 bytes)
        writer.write_all(&index_block_offset.to_le_bytes())?;
        writer.flush()?;

        Ok(())
    }

    // Open an existing SSTable and parse its index
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let file_len = file.metadata()?.len();

        if file_len < 8 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "SSTable file too small"));
        }

        // Read footer: index_block_offset (last 8 bytes)
        file.seek(SeekFrom::Start(file_len - 8))?;
        let mut footer_buf = [0u8; 8];
        file.read_exact(&mut footer_buf)?;
        let index_block_offset = u64::from_le_bytes(footer_buf);

        // Read index block
        file.seek(SeekFrom::Start(index_block_offset))?;
        
        // Compute total index block length
        let mut index_data = vec![0u8; (file_len - 8 - index_block_offset) as usize];
        file.read_exact(&mut index_data)?;

        let mut index = Vec::new();
        let mut cursor = 0usize;

        while cursor < index_data.len() {
            if cursor + 12 > index_data.len() {
                break;
            }
            let key_len = u32::from_le_bytes(index_data[cursor..cursor+4].try_into().unwrap()) as usize;
            let offset = u64::from_le_bytes(index_data[cursor+4..cursor+12].try_into().unwrap());
            cursor += 12;

            if cursor + key_len > index_data.len() {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Corrupted index block"));
            }
            let key = index_data[cursor..cursor+key_len].to_vec();
            cursor += key_len;

            index.push(IndexRecord { key, offset });
        }

        Ok(Self { file, index })
    }

    // Binary search the index and fetch value from disk
    pub fn get(&mut self, key: &[u8]) -> io::Result<Option<Option<Vec<u8>>>> {
        if self.index.is_empty() {
            return Ok(None);
        }

        // Binary search the in-memory index
        let search_res = self.index.binary_search_by(|record| record.key.as_slice().cmp(key));
        
        let record_idx = match search_res {
            Ok(idx) => idx,
            Err(_) => return Ok(None), // Since our index maps every single key, it must exist exactly if present
        };

        let offset = self.index[record_idx].offset;
        self.file.seek(SeekFrom::Start(offset))?;

        // Read record headers
        let mut len_buf = [0u8; 4];
        self.file.read_exact(&mut len_buf)?;
        let key_len = u32::from_le_bytes(len_buf) as usize;

        self.file.read_exact(&mut len_buf)?;
        let val_len_raw = u32::from_le_bytes(len_buf);

        // Read key
        let mut read_key = vec![0u8; key_len];
        self.file.read_exact(&mut read_key)?;

        if read_key != key {
            return Ok(None); // Index mapping mismatch
        }

        if val_len_raw == u32::MAX {
            // Tombstone: Key was explicitly deleted
            Ok(Some(None))
        } else {
            // Key is active: Read value
            let mut val = vec![0u8; val_len_raw as usize];
            self.file.read_exact(&mut val)?;
            Ok(Some(Some(val)))
        }
    }
}
