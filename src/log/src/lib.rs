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
}
