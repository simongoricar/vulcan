use std::path::PathBuf;

mod cli;
pub mod generation;
mod gui;

pub trait ExtendablePath {
    fn with_suffix_to_stem<S>(&self, suffix: S) -> Option<Self>
    where
        S: AsRef<str>,
        Self: Sized;
}

impl ExtendablePath for PathBuf {
    fn with_suffix_to_stem<S>(&self, suffix: S) -> Option<Self>
    where
        S: AsRef<str>,
        Self: Sized,
    {
        let file_stem = match self.file_stem() {
            Some(stem) => stem.to_string_lossy(),
            None => return None,
        };

        let file_extension = self.extension();

        Some(self.with_file_name(format!(
            "{stem}{suffix}{extension}",
            stem = file_stem,
            suffix = suffix.as_ref(),
            extension = match file_extension {
                Some(extension) => format!(".{}", extension.to_string_lossy()),
                None => String::from(""),
            }
        )))
    }
}
