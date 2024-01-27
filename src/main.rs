use std::io::prelude::*;
use std::io::{Seek, Write};
use std::iter::Iterator;
use zip::result::ZipError;
use zip::write::FileOptions;

use std::fs::File;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

fn main() {
    std::process::exit(real_main());
}

fn real_main() -> i32 {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        println!(
            "Usage: {} <source_directory>",
            args[0]
        );
        return 1;
    }

    let _ = iter_parent_dir(&*args[1]);

    0
}

fn iter_parent_dir(
    parent_dir: &str
) -> Result<(), ZipError> {
    let parent_path = Path::new(parent_dir);
    if !parent_path.is_dir() {
        return Err(ZipError::FileNotFound);
    }

    let parent_dir_name = parent_path.file_name().unwrap().to_str().unwrap();

    let walkdir = WalkDir::new(parent_dir);
    let it = walkdir.into_iter();

    for entry in it {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_dir() && entry.path() != parent_path {
                    let source: &Path = entry.path();
                    let filename = source.file_name().unwrap().to_str().unwrap();
                    let destination: &Path = &entry.path().with_file_name(format!("{parent_dir_name}{filename}")).with_extension("cbz");
                    match doit(source, destination, zip::CompressionMethod::Deflated) {
                        Ok(_) => println!("done: {source:?} written to {destination:?}"),
                        Err(e) => println!("Error compressing {source:?}: {e:?}"),
                    }
                }
            },
            Err(error) => {
                println!("Error iterating parent directory: {error:?}");
            }
        }
    }
    return Ok(());
    
}

fn zip_dir<T>(
    it: &mut dyn Iterator<Item = DirEntry>,
    prefix: &Path,
    writer: T,
    method: zip::CompressionMethod,
) -> zip::result::ZipResult<()>
where
    T: Write + Seek,
{
    let mut zip = zip::ZipWriter::new(writer);
    let options = FileOptions::default()
        .compression_method(method)
        .unix_permissions(0o755);

    let mut buffer = Vec::new();
    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(prefix).unwrap();

        // Write file or directory explicitly
        // Some unzip tools unzip files with directory paths correctly, some do not!
        if path.is_file() {
            println!("adding file {path:?} as {name:?} ...");
            #[allow(deprecated)]
            zip.start_file_from_path(name, options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear();
        } else if !name.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
            println!("adding dir {path:?} as {name:?} ...");
            #[allow(deprecated)]
            zip.add_directory_from_path(name, options)?;
        }
    }
    zip.finish()?;
    Result::Ok(())
}

fn doit(
    src_dir: &Path,
    dst_file: &Path,
    method: zip::CompressionMethod,
) -> zip::result::ZipResult<()> {
    if !src_dir.is_dir() {
        return Err(ZipError::FileNotFound);
    }

    let file = File::create(dst_file).unwrap();

    let walkdir = WalkDir::new(src_dir);
    let it = walkdir.into_iter();

    zip_dir(&mut it.filter_map(|e| e.ok()), src_dir, file, method)?;

    Ok(())
}