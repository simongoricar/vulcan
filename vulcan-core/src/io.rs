use std::{
    fs::OpenOptions,
    io::{self, BufWriter, Write},
    path::Path,
};

use image::{
    DynamicImage,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};
use thiserror::Error;


#[derive(Debug, Error)]
pub enum ImageSaveError {
    #[error("failed to open file for writing")]
    FileOpenError {
        #[source]
        error: io::Error,
    },

    #[error("failed to save")]
    ImageError {
        #[source]
        error: image::ImageError,
    },

    #[error("failed to flush buffered writer and close the file")]
    FileFlushError {
        #[source]
        error: io::Error,
    },
}


pub fn save_image_as_png<P>(
    image: &DynamicImage,
    file_path: P,
    overwrite_existing: bool,
) -> Result<(), ImageSaveError>
where
    P: AsRef<Path>,
{
    let file = if overwrite_existing {
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(file_path.as_ref())
    } else {
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(file_path.as_ref())
    }
    .map_err(|error| ImageSaveError::FileOpenError { error })?;

    let mut buf_writer = BufWriter::new(file);

    image
        .write_with_encoder(PngEncoder::new_with_quality(
            &mut buf_writer,
            CompressionType::Fast,
            FilterType::Adaptive,
        ))
        .map_err(|error| ImageSaveError::ImageError { error })?;

    let mut file = buf_writer.into_inner().map_err(|error| {
        ImageSaveError::FileFlushError {
            error: error.into_error(),
        }
    })?;

    file.flush()
        .map_err(|error| ImageSaveError::FileFlushError { error })?;
    drop(file);


    Ok(())
}
