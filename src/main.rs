extern crate blake2;
extern crate hex_slice;
extern crate walkdir;

use hex_slice::AsHex;
use std::io;
use std::env;
use std::io::prelude::*;
use std::fs::File;
use walkdir::WalkDir;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use blake2::{Blake2b, Digest};

fn hash_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut hasher = Blake2b::default();
    let mut f = File::open(&path)?;
    let mut buffer: Vec<u8> = Vec::new();
    f.read_to_end(&mut buffer)?;
    hasher.input(&buffer[..]);
    let digest = hasher.result().to_vec();

    Ok(digest)
}

fn list_dir<F>(path: &Path, mut callback: F) -> io::Result<()>
where
    F: FnMut(&Path, u64) -> (),
{
    for file in WalkDir::new(path)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if file.file_type().is_file() {
            callback(file.path(), file.metadata()?.len());
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let args_slice = &args[1..];
    let mut hashes: HashMap<Vec<u8>, Vec<PathBuf>> = HashMap::default();
    let mut sizes: HashMap<u64, Vec<PathBuf>> = HashMap::default();

    for argument in args_slice.iter() {
        let _ = list_dir(Path::new(&argument), |path, filesize| {
            let list = sizes.entry(filesize).or_insert(vec![]);
            list.push(path.to_path_buf());
        });
    }

    let mut duplicate_memory: u64 = 0;
    for (_size, files) in sizes.into_iter() {
        if files.len() > 1 {
            println!("Considering as duplicates (by size): {:?}", files);
            duplicate_memory = duplicate_memory + _size * (files.len() as u64 - 1);
            for path in files {
                if let Ok(digest) = hash_file(&path) {
                    let list = hashes.entry(digest).or_insert(vec![]);
                    list.push(path);
                }
            }
        } else {
            // debug:
            // println!("{} {:?}", size, files);
        }
    }

    println!("Removeable size: {:?}", duplicate_memory);

    for (hash, files) in hashes.iter() {
        println!("{:x} {:?}", (&hash[0..10]).as_hex(), files);
    }
}
