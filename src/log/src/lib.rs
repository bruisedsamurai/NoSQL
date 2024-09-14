

use std::fs::File;
use std::io;
use std::io::{BufReader, Read, Write};
use std::mem;
use std::path::Path;

//Both of the enums below constitute log record format

pub enum OpCode {
    Write = 0,
    Delete = 1,
}

pub enum Record {
    TypeValue(OpCode, usize, usize, String, String),
    TypeDelete(OpCode, usize, String),
}

pub struct LogWriter {
    writer: File,
}

type KeyType = u64;

impl LogWriter {
    pub fn new<P: AsRef<Path>>(path: P) -> LogWriter {
        LogWriter {
            writer: File::create(path).unwrap(),
        }
    }

    pub fn put(&mut self, key: &str, val: &str) -> io::Result<usize> {
        let mut vec: Vec<u8> = Vec::new();

        vec.push(OpCode::Write as u8);
        vec.append(&mut (key.len() as KeyType).to_be_bytes().to_vec());
        vec.append(&mut (val.len() as KeyType).to_be_bytes().to_vec());
        vec.append(&mut key.as_bytes().to_vec());
        vec.append(&mut val.as_bytes().to_vec());

        self.writer.write_all(&vec)?;
        self.writer.flush()?; //Even though writer puts all bytes in file instaneously. Still for extra surity flush is called.

        Ok(vec.len())
    }

    pub fn delete(&mut self, key: &str) -> io::Result<usize> {
        let mut vec: Vec<u8> = Vec::new();

        vec.push(OpCode::Delete as u8);
        vec.append(&mut (key.len() as KeyType).to_be_bytes().to_vec());
        vec.append(&mut key.as_bytes().to_vec());

        self.writer.write_all(&vec)?;
        self.writer.flush()?; //Even though writer puts all bytes in file instaneously. Still for extra surity flush is called.

        Ok(vec.len())
    }
}

pub struct LogReader {
    buffer_reader: BufReader<File>,
}

impl LogReader {
    const PAGE_SIZE: usize = 512;

    pub fn new<P: AsRef<Path>>(path: P) -> LogReader {
        let f = File::open(path).unwrap();
        LogReader {
            buffer_reader: BufReader::with_capacity(Self::PAGE_SIZE, f),
        }
    }
}

impl Iterator for LogReader {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        let mut opcode: [u8; 1] = [0; 1];

        self.buffer_reader.read_exact(&mut opcode[..]).unwrap();

        const KEY_SIZE: usize = mem::size_of::<KeyType>();
        let mut size_buff: [u8; KEY_SIZE] = [0; KEY_SIZE];

        const WRITE: u8 = OpCode::Write as u8;
        const DELETE: u8 = OpCode::Delete as u8;

        match opcode[0] {
            WRITE => {
                self.buffer_reader.read_exact(&mut size_buff).unwrap();

                let key_length = usize::from_be_bytes(size_buff);

                self.buffer_reader.read_exact(&mut size_buff).unwrap();

                let val_length = usize::from_be_bytes(size_buff);

                let mut buffer: Vec<u8> = vec![0; key_length];
                self.buffer_reader.read_exact(&mut buffer).unwrap();

                let key = std::str::from_utf8(&buffer).unwrap().to_string();

                buffer.resize(val_length, 0);
                self.buffer_reader.read_exact(&mut buffer).unwrap();

                let val = std::str::from_utf8(&buffer).unwrap().to_string();

                Some(Record::TypeValue(
                    OpCode::Write,
                    key_length,
                    val_length,
                    key,
                    val,
                ))
            }
            DELETE => {
                self.buffer_reader.read_exact(&mut size_buff).unwrap();

                let key_length = usize::from_be_bytes(size_buff);

                let mut buffer: Vec<u8> = vec![0; key_length];
                self.buffer_reader.read_exact(&mut buffer).unwrap();

                let key = std::str::from_utf8(&buffer).unwrap().to_string();

                Some(Record::TypeDelete(OpCode::Delete, key_length, key))
            }
            x => {
                panic!("Wrong index is read. Value at index: {}", x);
            }
        }
    }
}#[cfg(test)]
mod tests {

    use super::*;
    use rand::Rng;
    
    

    #[test]
    fn test_file_creation() {
        let path = "./00001.log";
        LogWriter::new(path);
        let result = File::open(path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_size_to_file() {
        let mut writer = LogWriter::new("./00001.log");
        let size = writer.put("foo", "bar").unwrap();

        assert_eq!(size, 1 + 8 + 8 + 3 + 3);
    }

    #[test]
    fn test_delete_size_to_file() {
        let mut writer = LogWriter::new("./00001.log");
        let size = writer.delete("foo").unwrap();

        assert_eq!(size, 1 + 8 + 3);
    }

    #[test]
    fn test_write_val_to_file() {
        let path = "./00001.log";
        let mut writer = LogWriter::new(path);
        writer.put("foo", "bar").unwrap();

        let mut f = File::open(path).unwrap();

        let mut buffer = vec![0; 23];
        f.read_exact(&mut buffer).unwrap();

        assert_eq!(buffer[0], 0);

        let mut slic: [u8; 8] = [0; 8];
        slic.clone_from_slice(&buffer[1..9]);
        assert_eq!(usize::from_be_bytes(slic), 3);

        let mut slic: [u8; 8] = [0; 8];
        slic.clone_from_slice(&buffer[9..17]);
        assert_eq!(usize::from_be_bytes(slic), 3);

        let mut slic: Vec<u8> = vec![0; 3];
        slic.clone_from_slice(&buffer[17..20]);
        assert_eq!(String::from_utf8(slic).unwrap(), "foo");

        let mut slic: Vec<u8> = vec![0; 3];
        slic.clone_from_slice(&buffer[20..23]);
        assert_eq!(String::from_utf8(slic).unwrap(), "bar");
    }

    #[test]
    fn test_sequence_write_val_to_file() {
        let path = "./00001.log";

        let sequence = [
            ("229427529247013", "9441423005"),
            ("59731486", "820063222306"),
            ("367312", "951"),
            ("7719318981985", "12075299017853"),
            ("815154270", "094903"),
            ("716584339405127", "1268"),
            ("71327", "8"),
        ];

        let mut writer = LogWriter::new(path);

        for (key, val) in sequence.iter() {
            writer.put(key, val).unwrap();
        }

        let mut f = File::open(path).unwrap();

        for (key, val) in sequence.iter() {
            let mut buffer = vec![0; 1 + 8 + 8 + key.len() + val.len()];
            f.read_exact(&mut buffer).unwrap();

            assert_eq!(buffer[0], 0);

            let mut slic: [u8; 8] = [0; 8];
            slic.clone_from_slice(&buffer[1..9]);
            assert_eq!(usize::from_be_bytes(slic), key.len());

            let mut slic: [u8; 8] = [0; 8];
            slic.clone_from_slice(&buffer[9..17]);
            assert_eq!(usize::from_be_bytes(slic), val.len());

            let mut slic: Vec<u8> = vec![0; key.len()];
            slic.clone_from_slice(&buffer[17..key.len()]);
            assert_eq!(String::from_utf8(slic).unwrap(), *key);

            let begin = 17 + key.len();
            let mut slic: Vec<u8> = vec![0; val.len()];
            slic.clone_from_slice(&buffer[begin..val.len()]);
            assert_eq!(String::from_utf8(slic).unwrap(), *val);
        }
    }

    #[test]
    fn test_delete_val_to_file() {
        let path = "./00001.log";
        let mut writer = LogWriter::new(path);
        writer.delete("foo").unwrap();

        let mut f = File::open(path).unwrap();

        let mut buffer = vec![0; 12];
        f.read_exact(&mut buffer).unwrap();

        assert_eq!(buffer[0], 1);

        let mut slic: [u8; 8] = [0; 8];
        slic.clone_from_slice(&buffer[1..9]);
        assert_eq!(usize::from_be_bytes(slic), 3);

        let mut slic: Vec<u8> = vec![0; 3];
        slic.clone_from_slice(&buffer[9..12]);
        assert_eq!(String::from_utf8(slic).unwrap(), "foo");
    }

    #[test]
    fn test_sequence_delete_val_to_file() {
        let path = "./00001.log";

        let sequence = [
            "229427529247013",
            "9441423005",
            "59731486",
            "820063222306",
            "367312",
            "951",
            "7719318981985",
            "12075299017853",
            "815154270",
            "094903",
            "716584339405127",
            "1268",
            "71327",
            "8",
        ];

        let mut writer = LogWriter::new(path);

        for key in sequence.iter() {
            writer.delete(key).unwrap();
        }

        let mut f = File::open(path).unwrap();

        for key in sequence.iter() {
            let mut buffer = vec![0; 1 + 8 + 8 + key.len()];
            f.read_exact(&mut buffer).unwrap();

            assert_eq!(buffer[0], 0);

            let mut slic: [u8; 8] = [0; 8];
            slic.clone_from_slice(&buffer[1..9]);
            assert_eq!(usize::from_be_bytes(slic), key.len());

            let mut slic: Vec<u8> = vec![0; key.len()];
            slic.clone_from_slice(&buffer[9..key.len()]);
            assert_eq!(String::from_utf8(slic).unwrap(), *key);
        }
    }

    #[test]
    fn test_iter_read_file() {
        let path = "./00001.log";

        let sequence = [
            ("229427529247013", "9441423005"),
            ("59731486", "820063222306"),
            ("367312", "951"),
            ("7719318981985", "12075299017853"),
            ("815154270", "094903"),
            ("716584339405127", "1268"),
            ("71327", "8"),
        ];

        let mut writer = LogWriter::new(path);

        let mut rng = rand::thread_rng();
        let mut op_vec = vec![];

        for (key, val) in sequence.iter() {
            let op_type: bool = rng.gen();

            if !op_type {
                writer.put(key, val).unwrap();
                op_vec.push(OpCode::Write);
            } else {
                writer.delete(key).unwrap();
                op_vec.push(OpCode::Delete);
            }
        }

        let reader = LogReader::new(path);

        for (i, rec) in reader.enumerate() {
            match op_vec[i] {
                OpCode::Write => {
                    let op_code: OpCode = OpCode::Delete;
                    let key_len: usize = 0;
                    let val_len: usize = 0;
                    let key: String = "".to_string();
                    let value: String = "".to_string();

                    assert!(matches!(
                        rec,
                        Record::TypeValue(op_code, key_len, val_len, key, value)
                    ));

                    assert!(matches!(op_code, OpCode::Write));
                    assert_eq!(key_len, sequence[i].0.len());
                    assert_eq!(val_len, sequence[i].1.len());
                    assert_eq!(key, sequence[i].0);
                    assert_eq!(value, sequence[i].1);
                }
                OpCode::Delete => {
                    let op_code: OpCode = OpCode::Write;
                    let key_len: usize = 0;
                    let key: String = "".to_string();

                    assert!(matches!(rec, Record::TypeDelete(op_code, key_len, key)));
                    assert!(matches!(op_code, OpCode::Write));
                    assert_eq!(key_len, sequence[i].0.len());
                    assert_eq!(key, sequence[i].0);
                }
            }
        }
    }
}
