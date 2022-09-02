use std::{env, process};

mod processor;
use processor::*;
mod input_file;
use input_file::*;
mod output_file;
use output_file::*;


fn main() {
    let args: Vec<String> = env::args().collect();
    
    // handle arguments
    if ( args.len() == 4 || args.len() == 5 ) && args[1] == "signature" {
        
        // check if chunk size was specified
        let chunk_size = if args.len() == 5 {
            match args[4].parse::<usize>() {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("Wrong value of chunk size: {}", &args[4]);
                    process::exit(1);
                }
            }
        } else {
            processor::CHUNK_SIZE
        };
        
        // try to open files
        let mut input_file = match InputFile::new(&args[2], chunk_size) {
            Ok(f) => f,
            Err(x) => {
                eprintln!("Unable to open inpug file: {}, error: {}", &args[2], x);
                process::exit(1);
            }
        };
        let mut output_file = match OutputFile::new(&args[3]) {
            Ok(f) => f,
            Err(x) => {
                eprintln!("Unable to create output file: {}, error: {}", &args[3], x);
                process::exit(1);
            }
        };
        
        // create logic processor
        let mut proc = Processor::new(&mut input_file, &mut output_file);

        // start processing input file to generate signature file
        if let Err(x) = proc.process_signature() {
            eprintln!("Processing error: {}", x);
        }
    }
    else if ( args.len() == 5 || args.len() == 6 ) && args[1] == "delta" {
        
        // check if chunk size was specified
        let chunk_size = if args.len() == 6 {            
            match args[5].parse::<usize>() {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("Wrong value of chunk size: {}", &args[5]);
                    process::exit(1);
                }
            }
        } else {
            processor::CHUNK_SIZE
        };
                
        // try to open files
        let mut input_file = match InputFile::new(&args[3], chunk_size) {
            Ok(f) => f,
            Err(x) => {
                eprintln!("Unable to open input file: {}, error: {}", &args[3], x);
                process::exit(1);
            }
        };
        let mut signature_file = match InputFile::new(&args[2], processor::HASH_SIZE) {
            Ok(f) => f,
            Err(x) => {
                eprintln!("Unable to open signature file: {}, error: {}", &args[2], x);
                process::exit(1);
            }
        };
        let mut output_file = match OutputFile::new(&args[4]) {
            Ok(f) => f,
            Err(x) => {
                eprintln!("Unable to create output file: {}, error: {}", &args[4], x);
                process::exit(1);
            }
        };

        // create logic processor
        let mut proc = Processor::new(&mut input_file, &mut output_file);

        // start processing input files to generate delta file
        if let Err(x) = proc.process_delta(&mut signature_file) {
            eprintln!("Processing error: {}", x);
        }        
        
        // delta file format:
        // 0 - current chank is same as in old file
        // 1 - apply new chunk which is added after this tag
        // 2 - chunk was inserted, value of the chunk is added after this tag
        // 3 - chunk was removed
    } else {
        // provide application usage info
        println!("Application usage:\nhdiff signature <input-file> <output-signature-file> [optional-chunk-size]\nhdiff delta <signature-file> <new-input-file> <output-delta-file> [optional-chunk-size]\n");
        process::exit(1);
    }
}
