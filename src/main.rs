extern crate blake2;
extern crate unbytify;
extern crate walkdir;

use std::io;
use std::env;
use std::io::prelude::*;
use std::fs::File;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};
use unbytify::*;
use blake2::{Blake2b, Digest};

const BUFFER_SIZE: usize = 4096;

fn hash_file(path: &Path, buffer: &mut [u8]) -> io::Result<Vec<u8>> {
    let mut hasher = Blake2b::default();
    let mut f = File::open(&path)?;

    loop {
        match f.read(buffer)? {
            0 => break,
            num => hasher.input(&buffer[..num]),
        }
    }

    let digest = hasher.result().to_vec();

    Ok(digest)
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn is_file(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
}

fn list_dir<F>(path: &Path, mut callback: F) -> io::Result<()>
where
    F: FnMut(&Path, u64) -> (),
{
    for file in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(|e| e.ok())
        .filter(|e| is_file(e))
    {
        callback(file.path(), file.metadata()?.len());
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let args_slice = &args[1..];
    let mut hashes: HashMap<Vec<u8>, (u64, Vec<PathBuf>)> = HashMap::default();
    let mut sizes: HashMap<u64, Vec<PathBuf>> = HashMap::default();
    let mut buffer = [0u8; BUFFER_SIZE];

    for argument in args_slice.iter() {
        let _ = list_dir(Path::new(&argument), |path, filesize| {
            let list = sizes.entry(filesize).or_insert_with(|| vec![]);
            list.push(path.to_path_buf());
        });
    }

    for (size, files) in sizes {
        if files.len() > 1 {
            // println!("Considering as duplicates (by size): {:?}", files);
            for path in files {
                if let Ok(digest) = hash_file(&path, &mut buffer) {
                    let list = hashes.entry(digest).or_insert_with(|| (size, vec![]));
                    list.1.push(path);
                }
            }
        } else {
            // debug:
            // println!("{} {:?}", size, files);
        }
    }

    let mut total = 0;

    for (_hash, (size, files)) in hashes {
        let length = files.len();
        if length > 1 {
            let duplicate_sum = size * (length as u64 - 1);
            total += duplicate_sum;
            let (num, unit) = bytify(duplicate_sum);
            println!("Duplicates: {} {}", num, unit);
            for file in files {
                println!("{}", file.display());
            }
            println!();
        }
    }

    if total > 0 {
        let (num, unit) = bytify(total);
        println!("Total: {} {} duplicated", num, unit);
    }
}
