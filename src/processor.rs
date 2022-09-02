use sha2::{Sha256, Digest};

// Default 1024 bytes chunk size
pub const CHUNK_SIZE: usize = 1024;

// Using SHA256 which gives 32 bytes hash size
pub const HASH_SIZE: usize = 32;

// tags for delta file
const TAG_SAME_HASH: [u8; 1] = [0]; 
const TAG_DIFFERENT_HASH: [u8; 1] = [1];
const TAG_INSERTED_CHUNK: [u8; 1] = [2];
const TAG_REMOVED_CHUNK: [u8; 1] = [3];


// Trait for input data
pub trait ProcessorDataInput {
    fn get_next_data(&mut self) -> &[u8];
    fn move_back_last_read(&mut self) -> bool; // true if success
}

// Trait for output data
pub trait ProcessorDataOutput {
    fn write_data(&mut self, data: &[u8]) -> bool; // true if success
}

// Custom error codes
pub enum ProcessorError {
    FileWrite,
    FileSeek
}
impl std::fmt::Display for ProcessorError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ProcessorError::FileWrite => write!(f, "File write error"),
            ProcessorError::FileSeek => write!(f, "Unable to change position in a file")
        }
    }
}

// Processor object
pub struct Processor<'a, T, S> {
    input_file: &'a mut T,
    output_file: &'a mut S
}


// Processor object implementation
impl<'a, T, S> Processor<'a, T, S> {
    
    // Constructor
    pub fn new(input_file: &'a mut T, output_file: &'a mut S) -> Self
        where T: ProcessorDataInput, S: ProcessorDataOutput
    {
        Self { input_file, output_file }
    }
    
    // Delta command logic
    pub fn process_delta(&mut self, signature_file: &mut T) -> Result<(), ProcessorError>
        where T: ProcessorDataInput, S: ProcessorDataOutput
    {
        loop {
            let mut input_file_chunk = self.input_file.get_next_data();
            if input_file_chunk.is_empty() { break } // reached end of file
                        
            let hash = calculate_chunk_hash(input_file_chunk);
            
            let sig_hash = signature_file.get_next_data();
            if sig_hash.is_empty() { 
                // end of signature file -> all data from input file needs to be added to delta
                while !input_file_chunk.is_empty() {
                    self.output_file.write_data(&TAG_DIFFERENT_HASH);                
                    self.output_file.write_data(input_file_chunk);
                
                    input_file_chunk = self.input_file.get_next_data();
                }
                break
            }

            if hash == sig_hash {
                // chunks are the same
                if !self.output_file.write_data(&TAG_SAME_HASH) {
                    return Err(ProcessorError::FileWrite)
                }
            } else {
                let input_file_chunk_prev = input_file_chunk.to_owned();
                let input_file_chunk = self.input_file.get_next_data();
                let hash_next = calculate_chunk_hash(input_file_chunk);
                
                let sig_hash_prev = sig_hash.to_owned();
                let sig_hash = signature_file.get_next_data();
                
                if sig_hash_prev == hash_next {
                    // current sigature hash is same as next input file hash -> previous chunk in new file was inserted
                    if !self.output_file.write_data(&TAG_INSERTED_CHUNK) || 
                       !self.output_file.write_data(&input_file_chunk_prev) ||
                       !self.output_file.write_data(&TAG_SAME_HASH) {
                        return Err(ProcessorError::FileWrite)
                    }
                    if !signature_file.move_back_last_read() {
                        return Err(ProcessorError::FileSeek)
                    }
                } else if sig_hash == hash {
                    // current input file hash is same as next sigature hash -> previous chunk in old file was removed
                    if !self.output_file.write_data(&TAG_REMOVED_CHUNK) || !self.output_file.write_data(&TAG_SAME_HASH) {
                        return Err(ProcessorError::FileWrite)
                    }
                    if !self.input_file.move_back_last_read() {
                        return Err(ProcessorError::FileSeek)
                    }
                } else {
                    // chunks are different
    
                    if !self.output_file.write_data(&TAG_DIFFERENT_HASH) || !self.output_file.write_data(&input_file_chunk_prev) {
                        return Err(ProcessorError::FileWrite)
                    }
                    if !self.input_file.move_back_last_read() || !signature_file.move_back_last_read() {
                        return Err(ProcessorError::FileSeek)
                    }
                }             
            }            
        }
        
        Ok(())
    }    
    
    // Signature command logic
    pub fn process_signature(&mut self) -> Result<(), ProcessorError>
        where T: ProcessorDataInput, S: ProcessorDataOutput
    {
        loop {            
            let input_file_chunk = self.input_file.get_next_data();
            if input_file_chunk.is_empty() { break } // reached end of file
             
            let hash = calculate_chunk_hash(input_file_chunk);
            
            if !self.output_file.write_data(&hash) {
                return Err(ProcessorError::FileWrite)
            }
        }
        
        Ok(())
    }
}

// Hash calculation
fn calculate_chunk_hash(chunk: &[u8]) -> [u8; HASH_SIZE] {
    let mut hasher = Sha256::new();
    hasher.update(chunk);
    let ret = hasher.finalize();
    ret.into()
}


// Processor tests
#[cfg(test)]
mod tests {
    use super::*;

    // helper object for testing processor
    struct MemData {
        data: Vec<u8>,
        location: usize,
        chunk_size: usize,
        last_read_size: usize
    }
    impl MemData {
        fn new_input( chunk_size: usize, data: &[u8] ) -> Self {
            Self { data: Vec::from(data), location: 0, chunk_size, last_read_size: 0 }
        }
        fn new_output() -> Self {
            Self { data: Vec::new(), location: 0, chunk_size: 0, last_read_size: 0 }
        }
    }
    impl ProcessorDataInput for MemData {
        fn get_next_data(&mut self) -> &[u8] {
            if self.location >= self.data.len() {
                self.data.clear();
                self.last_read_size = 0;
                &self.data
            } else if self.location + self.chunk_size >= self.data.len() {
                let ret = &self.data[self.location..];
                self.last_read_size = self.data.len() - self.location;
                self.location = self.data.len();
                ret
            } else {
                let ret = &self.data[self.location..self.location + self.chunk_size];
                self.last_read_size = self.chunk_size;
                self.location += self.chunk_size;               
                ret
            }
        }
        fn move_back_last_read(&mut self) -> bool {
            self.location -= self.last_read_size;
            true  
        }
    }
    impl ProcessorDataOutput for MemData {
        fn write_data(&mut self, data: &[u8]) -> bool {
            self.data.extend_from_slice(data);
            true
        }
    }

    #[test]
    fn test_sig_1() {
        // signature test
        // scenario: input file contains exactly 1 chunk
                
        let mut input = MemData::new_input(4, &[1,2,3,4]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_signature().is_ok() );

        let output_hash = [159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106];
        assert_eq!( output.data, output_hash );
    }

    #[test]
    fn test_sig_2() {
        // signature test
        // scenario: chunk size is larger than file size
        
        let mut input = MemData::new_input(10, &[1,2,3,4]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_signature().is_ok() );

        let output_hash = [159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106];
        assert_eq!( output.data, output_hash );
    }

    #[test]
    fn test_sig_3() {
        // signature test
        // scenario: input file consists of 2 same chunks
        
        let mut input = MemData::new_input(4, &[1,2,3,4,1,2,3,4]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_signature().is_ok() );

        let output_hash = [159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                           159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106];
        assert_eq!( output.data, output_hash );
    }

    #[test]
    fn test_sig_4() {
        // signature test
        // scenario: input file consists of 2 different chunks
        
        let mut input = MemData::new_input(4, &[1,2,3,4,5,6,7,8]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_signature().is_ok() );

        let output_hash = [159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                           85, 229, 80, 159, 128, 82, 153, 130, 148, 38, 110, 229, 181, 12, 181, 146, 147, 129, 145, 251, 93, 103, 247, 60, 172, 46, 96, 176, 39, 107, 27, 221];
        assert_eq!( output.data, output_hash );
    }
    
    #[test]
    fn test_sig_5() {
        // signature test
        // scenario: input file consists of 1 whole and 1 partial chunks
        
        let mut input = MemData::new_input(4, &[1,2,3,4,5,6]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_signature().is_ok() );

        let output_hash = [159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                           196, 37, 34, 18, 139, 73, 25, 61, 232, 205, 69, 216, 247, 88, 156, 215, 224, 133, 230, 95, 19, 134, 64, 213, 125, 68, 130, 229, 247, 24, 150, 35];
        assert_eq!( output.data, output_hash );
    }
    
    #[test]
    fn test_sig_6() {
        // signature test
        // scenario: input file is empty
        
        let mut input = MemData::new_input(4, &[]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_signature().is_ok() );

        assert_eq!( output.data, [] );
    }
    
    #[test]
    fn test_del_1() {
        // delta test
        // scenario: input file contains exactly 1 chunk, old file is same as input file
                
        let mut input = MemData::new_input(4, &[1,2,3,4]);
        let mut input_sig = MemData::new_input(HASH_SIZE, &[159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_delta(&mut input_sig).is_ok() );

        assert_eq!( output.data, [0] );
    }
    
    #[test]
    fn test_del_2() {
        // delta test
        // scenario: input file contains 2 chunks, old file is same as input file

        let mut input = MemData::new_input(4, &[1,2,3,4,1,2,3,4]);
        let mut input_sig = MemData::new_input(HASH_SIZE, &[159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_delta(&mut input_sig).is_ok() );

        assert_eq!( output.data, [0,0] );
    }
    
    #[test]
    fn test_del_3() {
        // delta test
        // scenario: input file contains 1 chunk, old file has 1 chunk different than new file
        
        let mut input = MemData::new_input(4, &[5,6,7,8]);
        let mut input_sig = MemData::new_input(HASH_SIZE, &[159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_delta(&mut input_sig).is_ok() );

        assert_eq!( output.data, [1,5,6,7,8] );
    }
    
    #[test]
    fn test_del_4() {
        // delta test
        // scenario: input file contains 2 chunks, old file contains 2 chunks 1st is same as in new file, 2nd is different

        let mut input = MemData::new_input(4, &[1,2,3,4,5,6,7,8]);
        let mut input_sig = MemData::new_input(HASH_SIZE, &[159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_delta(&mut input_sig).is_ok() );

        assert_eq!( output.data, [0,1,5,6,7,8] );
    }
        
    #[test]
    fn test_del_5() {
        // delta test
        // scenario: new file consists of 1 whole and 1 partial chunks, old file has same content
        
        let mut input = MemData::new_input(4, &[1,2,3,4,5,6]);
        let mut input_sig = MemData::new_input(HASH_SIZE, &[159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            196, 37, 34, 18, 139, 73, 25, 61, 232, 205, 69, 216, 247, 88, 156, 215, 224, 133, 230, 95, 19, 134, 64, 213, 125, 68, 130, 229, 247, 24, 150, 35]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_delta(&mut input_sig).is_ok() );

        assert_eq!( output.data, [0,0] );
    }
            
    #[test]
    fn test_del_6() {
        // delta test
        // scenario: new file consists of 1 whole and 1 partial chunks, old file has different 2nd chunk
        
        let mut input = MemData::new_input(4, &[1,2,3,4,5,6]);
        let mut input_sig = MemData::new_input(HASH_SIZE, &[159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_delta(&mut input_sig).is_ok() );

        assert_eq!( output.data, [0,1,5,6] );
    }
            
    #[test]
    fn test_del_7() {
        // delta test
        // scenario: new file consists of 1 whole and 1 partial chunks, old file has different 1st chunk
        
        let mut input = MemData::new_input(4, &[9,0,1,2,5,6]);
        let mut input_sig = MemData::new_input(HASH_SIZE, &[159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            196, 37, 34, 18, 139, 73, 25, 61, 232, 205, 69, 216, 247, 88, 156, 215, 224, 133, 230, 95, 19, 134, 64, 213, 125, 68, 130, 229, 247, 24, 150, 35]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_delta(&mut input_sig).is_ok() );

        assert_eq!( output.data, [1,9,0,1,2,0] );
    }
    
    #[test]
    fn test_del_8() {
        // delta test
        // scenario: new file has added 2nd chunks at the end (chunk size: 4)
        // old file: 1,2,3,4, 1,2,3,4
        // new file: 1,2,3,4, 1,2,3,4, 1,2,3,4, 5,6,7,8
                
        let mut input = MemData::new_input(4, &[1,2,3,4,1,2,3,4,1,2,3,4,5,6,7,8]);
        let mut input_sig = MemData::new_input(HASH_SIZE, &[159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_delta(&mut input_sig).is_ok() );

        assert_eq!( output.data, [0,0,1,1,2,3,4,1,5,6,7,8] );
    }
    
    #[test]
    fn test_del_9() {
        // delta test
        // scenario: new file is completely different than old file (chunk size: 4)
        // old file: 1,2,3,4, 1,2,3,4, 1,2,3,4
        // new file: 5,6,7,8, 5,6,7,8, 5,6,7,8, 5,6,7,8
                
        let mut input = MemData::new_input(4, &[5,6,7,8,5,6,7,8,5,6,7,8,5,6,7,8]);
        let mut input_sig = MemData::new_input(HASH_SIZE, &[159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_delta(&mut input_sig).is_ok() );

        assert_eq!( output.data, [1,5,6,7,8,1,5,6,7,8,1,5,6,7,8,1,5,6,7,8] );
    }
    
    #[test]
    fn test_del_10() {
        // delta test
        // scenario: new file is completely different than old file (chunk size: 4)
        // old file: 1,2,3,4, 1,2,3,4, 9,0,1,2, 1,2,3,4, 1,2,3,4, 5,6
        // new file: 1,2,3,4, 1,2,3,4, 5,6,7,8, 1,2,3,4, 1,2,3,4, 5,6
                
        let mut input = MemData::new_input(4, &[1,2,3,4,1,2,3,4,5,6,7,8,1,2,3,4,1,2,3,4,5,6]);
        let mut input_sig = MemData::new_input(HASH_SIZE, &[159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            15, 196, 39, 34, 18, 139, 73, 25, 61, 232, 205, 69, 216, 247, 88, 156, 215, 224, 133, 230, 95, 19, 134, 64, 213, 125, 68, 130, 229, 247, 24, 150,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            196, 37, 34, 18, 139, 73, 25, 61, 232, 205, 69, 216, 247, 88, 156, 215, 224, 133, 230, 95, 19, 134, 64, 213, 125, 68, 130, 229, 247, 24, 150, 35]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_delta(&mut input_sig).is_ok() );

        assert_eq!( output.data, [0,0,1,5,6,7,8,0,0,0] );
    }
    
    #[test]
    fn test_del_insert_1() {
        // delta test
        // scenario: new file has added chunk between 1st and 2nd chunks in old file (chunk size: 4)
        // old file: 1,2,3,4, 1,2,3,4
        // new file: 1,2,3,4, 5,6,7,8, 1,2,3,4

        let mut input = MemData::new_input(4, &[1,2,3,4,5,6,7,8,1,2,3,4]);
        let mut input_sig = MemData::new_input(HASH_SIZE, &[159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_delta(&mut input_sig).is_ok() );

        assert_eq!( output.data, [0,2,5,6,7,8,0] );
    }
    
    #[test]
    fn test_del_insert_2() {
        // delta test
        // scenario: new file has added chunk between 1st and 2nd chunks in old file (chunk size: 4)
        // old file: 1,2,3,4, 1,2,3,4, 1,2,3,4
        // new file: 1,2,3,4, 5,6,7,8, 1,2,3,4, 1,2,3,4

        let mut input = MemData::new_input(4, &[1,2,3,4,5,6,7,8,1,2,3,4,1,2,3,4]);
        let mut input_sig = MemData::new_input(HASH_SIZE, &[159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_delta(&mut input_sig).is_ok() );

        assert_eq!( output.data, [0,2,5,6,7,8,0,0] );
    }
    
    #[test]
    fn test_del_remove_1() {
        // delta test
        // scenario: new file has removed 2nd chunk from old file (chunk size: 4)
        // old file: 1,2,3,4, 5,6,7,8, 1,2,3,4
        // new file: 1,2,3,4, 1,2,3,4
                
        let mut input = MemData::new_input(4, &[1,2,3,4,1,2,3,4]);
        let mut input_sig = MemData::new_input(HASH_SIZE, &[159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            85, 229, 80, 159, 128, 82, 153, 130, 148, 38, 110, 229, 181, 12, 181, 146, 147, 129, 145, 251, 93, 103, 247, 60, 172, 46, 96, 176, 39, 107, 27, 221,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_delta(&mut input_sig).is_ok() );

        assert_eq!( output.data, [0,3,0] );
    }
    
    #[test]
    fn test_del_remove_2() {
        // delta test
        // scenario: new file has removed 2nd chunk from old file and two more same chunks (chunk size: 4)
        // old file: 1,2,3,4, 5,6,7,8, 1,2,3,4, 1,2,3,4
        // new file: 1,2,3,4, 1,2,3,4, 1,2,3,4
                
        let mut input = MemData::new_input(4, &[1,2,3,4,1,2,3,4,1,2,3,4]);
        let mut input_sig = MemData::new_input(HASH_SIZE, &[159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            85, 229, 80, 159, 128, 82, 153, 130, 148, 38, 110, 229, 181, 12, 181, 146, 147, 129, 145, 251, 93, 103, 247, 60, 172, 46, 96, 176, 39, 107, 27, 221,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106,
                                                            159, 100, 167, 71, 225, 185, 127, 19, 31, 171, 182, 180, 71, 41, 108, 155, 111, 2, 1, 231, 159, 179, 197, 53, 110, 108, 119, 232, 155, 106, 128, 106]);
        let mut output = MemData::new_output();

        let mut proc = Processor::new(&mut input, &mut output);
        assert!( proc.process_delta(&mut input_sig).is_ok() );

        assert_eq!( output.data, [0,3,0,0] );
    }
}

