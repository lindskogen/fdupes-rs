extern crate blake2;
#[macro_use]
extern crate clap;
extern crate rayon;
extern crate unbytify;
extern crate walkdir;

use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use clap::{App, Arg};
use walkdir::{DirEntry, WalkDir};
use unbytify::*;
use blake2::{Blake2b, Digest};
use rayon::prelude::*;

const BUFFER_SIZE: usize = 4096;

fn hash_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut hasher = Blake2b::default();
    let mut f = File::open(&path)?;

    loop {
        match f.read(&mut buffer)? {
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

fn list_dir<F>(path: &Path, max_depth: usize, mut callback: F) -> io::Result<()>
where
    F: FnMut(&Path, u64) -> (),
{
    for file in WalkDir::new(path)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        // .filter_entry(|e| !is_hidden(e))
        .filter_map(|e| e.ok())
        .filter(|e| is_file(e))
    {
        callback(file.path(), file.metadata()?.len());
    }
    Ok(())
}

fn main() {
    let matches = App::new("fdupes-rs")
        .version("1.0")
        .author("Johan Lindskogen <johan.lindskogen@gmail.com>")
        .about("Find duplicated files")
        .arg(
            Arg::with_name("summarize")
                .short("m")
                .long("summarize")
                .help("Summarize size information"),
        )
        .arg(
            Arg::with_name("max depth")
                .short("d")
                .long("depth")
                .help("Max depth to recurse down in directories")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("FILE")
                .help("Input files or folders")
                .required(true)
                .multiple(true)
                .index(1),
        )
        .get_matches();

    let files = matches.values_of("FILE").unwrap();
    let max_depth = value_t!(matches.value_of("max depth"), usize).unwrap_or(::std::usize::MAX);
    let summarize = matches.is_present("summarize");

    let mut hashes: HashMap<Vec<u8>, (u64, Vec<PathBuf>)> = HashMap::default();
    let mut sizes: HashMap<u64, Vec<PathBuf>> = HashMap::default();

    for argument in files {
        let _ = list_dir(Path::new(&argument), max_depth, |path, filesize| {
            let list = sizes.entry(filesize).or_insert_with(|| vec![]);
            list.push(path.to_path_buf());
        });
    }

    let hashtriple: Vec<(u64, &PathBuf, Vec<u8>)> = sizes
        .par_iter()
        .filter(|&(_, files)| files.len() > 1)
        .flat_map(|(size, files)| {
            files
                .into_par_iter()
                .filter_map(move |path| hash_file(path).map(|digest| (*size, path, digest)).ok())
        })
        .collect();

    for (size, path, digest) in hashtriple {
        let list = hashes.entry(digest).or_insert_with(|| (size, vec![]));
        list.1.push(path.to_path_buf());
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

    if summarize {
        let (num, unit) = bytify(total);
        println!("Total: {} {} duplicated", num, unit);
    }
}
