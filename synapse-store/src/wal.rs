use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write, BufWriter};
use std::path::Path;

#[derive(Debug, Clone)]
pub enum LogOp {
    Put(Vec<u8>, Vec<u8>),
    Delete(Vec<u8>),
}

pub struct WriteAheadLog {
    writer: BufWriter<File>,
}

impl WriteAheadLog {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    pub fn append_put(&mut self, key: &[u8], val: &[u8]) -> io::Result<()> {
        // Write Format: [Op Code: 1] [Key Len: 4] [Val Len: 4] [Key] [Val]
        self.writer.write_all(&[1])?; // 1 = Put
        self.writer.write_all(&(key.len() as u32).to_le_bytes())?;
        self.writer.write_all(&(val.len() as u32).to_le_bytes())?;
        self.writer.write_all(key)?;
        self.writer.write_all(val)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn append_delete(&mut self, key: &[u8]) -> io::Result<()> {
        // Write Format: [Op Code: 1] [Key Len: 4] [Key]
        self.writer.write_all(&[2])?; // 2 = Delete
        self.writer.write_all(&(key.len() as u32).to_le_bytes())?;
        self.writer.write_all(key)?;
        self.writer.flush()?;
        Ok(())
    }

    // Parse the WAL file on database recovery
    pub fn recover<P: AsRef<Path>>(path: P) -> io::Result<Vec<LogOp>> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Ok(Vec::new());
        }

        let mut file = File::open(path_ref)?;
        let mut ops = Vec::new();
        let mut op_buf = [0u8; 1];
        let mut len_buf = [0u8; 4];

        while file.read_exact(&mut op_buf).is_ok() {
            let op_code = op_buf[0];
            match op_code {
                1 => { // Put
                    file.read_exact(&mut len_buf)?;
                    let key_len = u32::from_le_bytes(len_buf) as usize;
                    file.read_exact(&mut len_buf)?;
                    let val_len = u32::from_le_bytes(len_buf) as usize;

                    let mut key = vec![0u8; key_len];
                    file.read_exact(&mut key)?;
                    let mut val = vec![0u8; val_len];
                    file.read_exact(&mut val)?;

                    ops.push(LogOp::Put(key, val));
                }
                2 => { // Delete
                    file.read_exact(&mut len_buf)?;
                    let key_len = u32::from_le_bytes(len_buf) as usize;

                    let mut key = vec![0u8; key_len];
                    file.read_exact(&mut key)?;

                    ops.push(LogOp::Delete(key));
                }
                _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid WAL op code")),
            }
        }

        Ok(ops)
    }
}
