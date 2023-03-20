use std::{fs::File, io::{BufReader, BufRead}};
use flate2::read::GzDecoder;

fn main() { 
    let file_path = "EW88RAZPPLNFH.2022-06-03-04.eb916fad.gz";
    let gz_file = File::open(file_path).expect("Ooops.");
    let compressed_reader = BufReader::new(gz_file);
    let archive = GzDecoder::new(compressed_reader);
    let mut reader_actual = BufReader::new(archive);

    let mut line = String::new();

    while reader_actual.read_line(&mut line).unwrap() != 0 {
        println!("{line}");
        line.clear();
    }

}
