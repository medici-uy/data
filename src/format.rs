use std::path::PathBuf;

use anyhow::Result;

use medici_data_sync::{read_data_dir, CourseData};

pub async fn format(data_path: PathBuf, images_path: PathBuf) -> Result<()> {
    let entries = read_data_dir(data_path)?;

    for dir_entry in entries {
        CourseData::load_and_write_formatted(dir_entry?, images_path.clone()).await?;
    }

    Ok(())
}
