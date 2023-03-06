use super::constants;
use color_eyre::{eyre::Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{
    fs::{DirBuilder, File},
    io::{BufReader, Write},
    path::Path,
};
use chrono::{DateTime, Utc};
use crate::cache::ZorkCache;

pub fn create_file<'a>(path: &Path, filename: &'a str, buff_write: &'a [u8]) -> Result<()> {
    let file_path = path.join(filename);

    File::create(&file_path)
        .with_context(|| format!("Could not create file {file_path:?}"))?
        .write_all(buff_write)
        .with_context(|| format!("Could not write to file {file_path:?}"))
}

pub fn create_directory(path_create: &Path) -> Result<()> {
    DirBuilder::new()
        .recursive(true)
        .create(path_create)
        .with_context(|| format!("Could not create directory {path_create:?}"))
}

/// Gets the absolute route for an element in the system given a path P,
/// without the extension is P belongs to a file
pub fn get_absolute_path<P: AsRef<Path>>(p: P) -> PathBuf {
    let mut canonical = p
        .as_ref()
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(p.as_ref()));
    if cfg!(target_os = "windows") {
        canonical = canonical
            .to_str()
            .map(|unc| &unc[4..])
            .unwrap_or_default()
            .into()
    }
    let file_stem = canonical.file_stem().unwrap_or_default();
    let r = canonical
        .parent()
        .unwrap_or_else(|| panic!("Unexpected error getting the parent of {:?}", p.as_ref()))
        .join(file_stem);
    log::trace!(
        "Generated file: {:?}, file stem: {file_stem:?}, and canonical: {canonical:?}",
        &r
    );
    r
}

/// Returns true if the file changed since the last time that Zork++ made a build process,
/// false otherwise.
pub fn did_file_changed_since_last_run(cache: &ZorkCache, file: &Path) -> Option<bool> {
    let last_process_timestamp = cache.last_program_execution;
    let file_metadata = file.metadata();
    match file_metadata {
        Ok(m) => match m.modified() {
            Ok(modified) => Some(DateTime::<Utc>::from(modified) < last_process_timestamp),
            Err(e) => {
                log::error!("An error happened trying to get the last time that the {file:?} was modified. Processing it anyway because {e:?}");
                None
            }
        },
        Err(e) => {
            log::error!("An error happened trying to retrieve the metadata of {file:?}. Processing it anyway because {e:?}");
            None
        }
    }
}

/// Returns the declared extension for a file, if exists
#[inline(always)]
pub fn get_file_extension<P: AsRef<Path>>(p: P) -> String {
    p.as_ref()
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_string()
}

pub fn serialize_object_to_file<T>(path: &Path, data: &T) -> Result<()>
where
    T: Serialize,
{
    serde_json::to_writer_pretty(
        File::create(path).with_context(|| "Error creating the cache file")?,
        data,
    )
    .with_context(|| "Error serializing data to the cache")
}

pub fn load_and_deserialize<T, P>(path: &P) -> Result<T>
where
    T: for<'a> Deserialize<'a> + Default,
    P: AsRef<Path>,
{
    let buffer = BufReader::new(
        File::open(path.as_ref().join(constants::ZORK_CACHE_FILENAME))
            .with_context(|| "Error opening the cache file")?,
    );
    Ok(serde_json::from_reader(buffer).unwrap_or_default())
}
