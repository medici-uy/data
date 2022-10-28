use std::{ffi::OsStr, path::PathBuf};

use anyhow::Result;

pub trait Hashable {
    fn hashable_data(&self) -> Vec<u8>;
    fn set_hash(&mut self);

    fn hash_data(&self) -> String {
        blake3::hash(&self.hashable_data()).to_string()
    }
}

#[async_trait::async_trait]
pub trait WithImage: CourseAssociated + Sync {
    fn current_image_file_name(&self) -> Option<&PathBuf>;
    fn canonical_image_file_name(&self) -> String;
    fn replace_image_file_name(&mut self, new_file_name: PathBuf);

    async fn format_image(&mut self, images_path: PathBuf) -> Result<()> {
        if self.current_image_file_name().is_some() {
            if let Some(new_file_name) = self.rename_image(images_path).await? {
                self.replace_image_file_name(new_file_name);
            }
        }

        Ok(())
    }

    async fn rename_image(&self, images_path: PathBuf) -> Result<Option<PathBuf>> {
        let current_image_file_name = self.current_image_file_name().unwrap();

        let stem = current_image_file_name
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap();

        if stem != self.canonical_image_file_name() {
            let extension = current_image_file_name
                .extension()
                .and_then(OsStr::to_str)
                .expect("no extension in image file name");

            let mut new_file_name = PathBuf::from(self.canonical_image_file_name());
            new_file_name.set_extension(extension);

            let mut old_path = images_path.clone();
            old_path.push(current_image_file_name);
            let mut new_path = images_path.clone();
            new_path.push(new_file_name.clone());

            tokio::fs::rename(old_path, new_path).await?;

            Ok(Some(new_file_name))
        } else {
            Ok(None)
        }
    }

    fn full_image_path(&self) -> Option<String> {
        Some(format!(
            "{}/{}",
            self.course_key(),
            self.current_image_file_name()?
                .as_os_str()
                .to_string_lossy()
        ))
    }
}

pub trait CourseAssociated {
    fn course_key(&self) -> &str;
    fn set_course_key(&mut self, course_key: String);
}
