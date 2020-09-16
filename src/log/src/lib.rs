#[cfg(test)]
mod tests {}

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

struct LogWriter {
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

        vec.push(1);
        vec.append(&mut (key.len() as KeyType).to_be_bytes().to_vec());
        vec.append(&mut (val.len() as KeyType).to_be_bytes().to_vec());
        vec.append(&mut key.as_bytes().to_vec());
        vec.append(&mut val.as_bytes().to_vec());

        self.writer.write(&vec)
    }

    pub fn delete(&mut self, key: &str) -> io::Result<usize> {
        let mut vec: Vec<u8> = Vec::new();

        vec.push(0);
        vec.append(&mut (key.len() as KeyType).to_be_bytes().to_vec());
        vec.append(&mut key.as_bytes().to_vec());

        self.writer.write(&vec)
    }

    pub fn read<P: AsRef<Path>>(path: P) -> LogReader {
        let f = File::open(path).unwrap();

        LogReader::new(f)
    }
}

struct LogReader {
    buffer_reader: BufReader<File>,
}

impl LogReader {
    const PAGE_SIZE: usize = 512;

    pub fn new(f: File) -> LogReader {
        LogReader {
            buffer_reader: BufReader::with_capacity(Self::PAGE_SIZE, f),
        }
    }
}

impl Iterator for LogReader {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        let mut opcode: [u8; 1] = [0; 1];
        let mut n;

        n = self.buffer_reader.read(&mut opcode[..]).unwrap();

        if n == 0 {
            return None;
        }

        const KEY_SIZE: usize = mem::size_of::<KeyType>();
        let mut size_buff: [u8; KEY_SIZE] = [0; KEY_SIZE];

        match opcode[0] {
            0 => {
                n = self.buffer_reader.read(&mut size_buff).unwrap();

                assert_ne!(n, 0, "Corrupt data");
                assert_eq!(n, KEY_SIZE);

                let key_length = usize::from_be_bytes(size_buff);

                n = self.buffer_reader.read(&mut size_buff).unwrap();
                if n == 0 {
                    panic!("Corrupt data");
                }
                let val_length = usize::from_be_bytes(size_buff);

                let mut buffer: Vec<u8> = vec![0; key_length];
                n = self.buffer_reader.read(&mut buffer).unwrap();
                assert_ne!(n, 0, "Corrupt data");
                assert_eq!(n, key_length);

                let key = std::str::from_utf8(&buffer).unwrap().to_string();

                buffer.resize(val_length, 0);
                n = self.buffer_reader.read(&mut buffer).unwrap();
                assert_ne!(n, 0, "Corrupt data");
                assert_eq!(n, val_length);

                let val = std::str::from_utf8(&buffer).unwrap().to_string();

                Some(Record::TypeValue(
                    OpCode::Write,
                    key_length,
                    val_length,
                    key,
                    val,
                ))
            }
            1 => {
                n = self.buffer_reader.read(&mut size_buff).unwrap();

                assert_ne!(n, 0, "Corrupt data");
                assert_eq!(n, KEY_SIZE);

                let key_length = usize::from_be_bytes(size_buff);

                let mut buffer: Vec<u8> = vec![0; key_length];
                n = self.buffer_reader.read(&mut buffer).unwrap();
                assert_ne!(n, 0, "Corrupt data");
                assert_eq!(n, key_length);

                let key = std::str::from_utf8(&buffer).unwrap().to_string();

                Some(Record::TypeDelete(OpCode::Delete, key_length, key))
            }
            x => {
                panic!("Wrong index is read. Value at index: {}", x);
            }
        }
    }
}
