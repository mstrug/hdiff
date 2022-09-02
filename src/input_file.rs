use std::{error::Error, io::Read, convert::TryFrom};
use super::processor::*;



pub struct InputFile {
    reader: std::io::BufReader<std::fs::File>,
    chunk_size: usize,
    chunk: Vec<u8>,
    len_to_read: u64
}

impl InputFile {
    
    pub fn new(file_name: &str, chunk_size: usize) -> Result<Self, Box<dyn Error>> {
        let file = std::fs::File::open(file_name)?;
        let metadata = file.metadata()?;
        let reader = std::io::BufReader::new(file);
        let mut chunk: Vec<u8> = Vec::new();
        chunk.resize(chunk_size, 0);
        Ok( Self { reader, chunk_size, chunk, len_to_read: metadata.len() } )
    }
    
}

impl ProcessorDataInput for InputFile {
    fn get_next_data(&mut self) -> &[u8] {
        
        if self.len_to_read == 0 {
            self.chunk.clear(); 
            return &self.chunk
        }
        else if self.len_to_read < self.chunk_size as u64 {
            if let Ok(val) = usize::try_from(self.len_to_read) {
                self.chunk.truncate(val);
            } else {
                // error case
                self.chunk.clear(); 
                return &self.chunk
            }
        }
        
        match self.reader.read_exact(&mut self.chunk) {
            Ok(()) => {
                self.len_to_read -= self.chunk.len() as u64;
                &self.chunk
            }
            Err(_) => {
                // in case of any error return empty array
                self.chunk.clear();
                &self.chunk
            }
        }
    }
    
    fn move_back_last_read(&mut self) -> bool {
        self.len_to_read += self.chunk.len() as u64;
        self.reader.seek_relative(-(self.chunk.len() as i64)).is_ok()
    }
}

