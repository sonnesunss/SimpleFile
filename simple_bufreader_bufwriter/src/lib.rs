/*
    实现一个简单的BufReader and BufWriter
*/

use std::io;
use std::usize;

use simple_file::File;

#[allow(dead_code)]
pub struct BufReader {
    file: File,
    buffer: Vec<u8>,
    pos: usize,
    capacity: usize,
}

impl BufReader {
    pub fn new(file: File) -> BufReader {
        const BUFFER_SIZE: usize = 4096; // 4KB 缓冲区

        BufReader {
            file,
            buffer: vec![0; BUFFER_SIZE],
            pos: 0,
            capacity: 0,
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let mut total_read = 0;
        while total_read < buf.len() {
            if self.pos >= self.capacity {
                self.pos = 0;
                self.capacity = self.file.read(&mut self.buffer)?;
                if self.capacity == 0 {
                    return Ok(total_read);
                }
            }

            let to_copy = std::cmp::min(self.capacity - self.pos, buf.len() - total_read);

            buf[total_read..total_read + to_copy]
                .copy_from_slice(&self.buffer[self.pos..self.pos + to_copy]);
            self.pos += to_copy;
            total_read += to_copy;
        }
        Ok(total_read)
    }

    pub fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        buf.clear();

        let mut total_read = 0;

        loop {
            if self.pos >= self.capacity {
                self.pos = 0;
                self.capacity = self.file.read(&mut self.buffer)?;
                if self.capacity == 0 {
                    return Ok(total_read);
                }
            }

            // 查找换行符
            let start = self.pos;
            let end = self.buffer[self.pos..self.capacity]
                .iter()
                .position(|&b| b == b'\n')
                .map(|i| self.pos + i + 1)
                .unwrap_or(self.capacity);
            let slice = &self.buffer[self.pos..end];
            buf.push_str(std::str::from_utf8(slice).map_err(|_| {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid UTF-8 data",
                ));
            })?);
            total_read += end - self.pos;
            self.pos = end;

            if end < self.capacity || self.buffer[end - 1] == b'\n' {
                return Ok(total_read);
            }
        }
    }
}

#[allow(dead_code)]
pub struct BufWriter {
    file: File,
    buffer: Vec<u8>,
    pos: usize,
    capacity: usize,
}
