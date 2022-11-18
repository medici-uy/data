use std::fs::{self, DirEntry, ReadDir};
use std::path::PathBuf;

use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::CourseData;

static WHITESPACE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s\s+").unwrap());

pub fn read_data_dir(data_path: PathBuf) -> Result<ReadDir> {
    let data_path = fs::canonicalize(data_path)?;
    let entries = fs::read_dir(data_path)?;

    Ok(entries)
}

pub fn read_dir_entry_data(dir_entry: DirEntry) -> Result<Vec<u8>> {
    if dir_entry.file_type()?.is_dir() {
        bail!("unexpected directory");
    };

    Ok(fs::read(dir_entry.path())?)
}

pub fn write_data(path: PathBuf, data: String) -> Result<()> {
    fs::write(path, format!("{data}\n"))?;

    Ok(())
}

pub async fn load_courses_data_and_write_formatted(
    data_path: PathBuf,
    images_path: PathBuf,
) -> Result<Vec<CourseData>> {
    let mut courses_data = vec![];

    for dir_entry in read_data_dir(data_path)? {
        courses_data
            .push(CourseData::load_and_write_formatted(dir_entry?, images_path.clone()).await?);
    }

    Ok(courses_data)
}

pub fn format_text(text: &str) -> String {
    let mut formatted = text.trim().to_owned();

    formatted = WHITESPACE_REGEX.replace_all(&formatted, " ").to_string();

    formatted
}
