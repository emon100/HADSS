use std::{fs, io};

use crate::ARGS;

pub type DirectoryPath = String;
pub type Filename = String;

/// This function splits file id into tuple(DirectoryPath, Filename)
/// e.g. split_id_into_directory_and_filename("1234567890", 3) -> ("12/34/56","7890")
fn split_id_into_directory_and_filename(id: &str,
                                        directory_depth: usize,
                                        directory_name_length: usize)
                                        -> (DirectoryPath, Filename) {
    let chunks: Vec<&str> = id.as_bytes()
                              .chunks(directory_name_length)
                              .map(|x| std::str::from_utf8(x).unwrap())
                              .collect();

    let directory: String = chunks
        .iter()
        .take(directory_depth)
        .cloned()
        .intersperse("/")
        .collect();

    let filename: String = chunks
        .iter()
        .skip(directory_depth)
        .cloned()
        .intersperse("")
        .collect();

    (directory, filename)
}

pub fn store_slice(id: &str, body: &Vec<u8>) -> io::Result<()> {
    let storage_directory_depth: usize = ARGS.storage_directory_depth;
    let (directory, filename) = split_id_into_directory_and_filename(id, storage_directory_depth, 2);

    let full_directory = format!("{}/{}", ARGS.storage_location, directory);
    fs::create_dir_all(&full_directory)?;

    let full_path = format!("{}/{}", full_directory, filename);
    println!("{}", full_path);
    fs::write(full_path, body)
}

pub fn read_slice(id: &str) -> io::Result<Vec<u8>> {
    let storage_directory_depth: usize = ARGS.storage_directory_depth;
    let (directory, filename) = split_id_into_directory_and_filename(id, storage_directory_depth, 2);

    let path = format!("{}/{}/{}", ARGS.storage_location, directory, filename);
    println!("{}", path);
    fs::read(path)
}
