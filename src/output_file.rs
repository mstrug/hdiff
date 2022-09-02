use std::{error::Error, io::Write};
use super::processor::*;



pub struct OutputFile {
    writer: std::io::BufWriter<std::fs::File>
}

impl OutputFile {
    
    pub fn new(file_name: &str) -> Result<Self, Box<dyn Error>> {
        let file = std::fs::File::create(file_name)?;
        let writer = std::io::BufWriter::new(file);
        Ok( Self { writer } )
    }
    
}

impl ProcessorDataOutput for OutputFile {
    fn write_data(&mut self, data: &[u8]) -> bool {
        self.writer.write(data).is_ok()
    }
}

