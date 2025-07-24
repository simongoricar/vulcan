use std::{fs::OpenOptions, io, path::Path};

use image::{
    DynamicImage,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};
use thiserror::Error;


#[derive(Debug, Error)]
enum ImageSaveError {
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
}


fn save_image_as_png<I, P>(
    image: I,
    file_path: P,
    overwrite_existing: bool,
) -> Result<(), ImageSaveError>
where
    I: Into<DynamicImage>,
    P: AsRef<Path>,
{
    let file = if overwrite_existing {
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(file_path.as_ref())
    } else {
        OpenOptions::new().create_new(true).open(file_path.as_ref())
    }
    .map_err(|error| ImageSaveError::FileOpenError { error })?;

    let dynamic_image: DynamicImage = image.into();

    dynamic_image
        .write_with_encoder(PngEncoder::new_with_quality(
            file,
            CompressionType::Best,
            FilterType::Adaptive,
        ))
        .map_err(|error| ImageSaveError::ImageError { error })?;

    Ok(())
}
