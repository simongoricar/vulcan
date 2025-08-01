use std::{
    fs,
    io,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use image::{DynamicImage, RgbaImage};
use thiserror::Error;
use vulcan_core::{
    feedback::{FeedbackSegmentSelectionMode, PIXEL_BLACK, mask_out_non_targeted_pixels},
    io::{ImageSaveError, save_image_as_png},
    pixel_sorting::{
        ImageSortingDirection,
        immediate::{ImmediateSegmentSelectionMode, PixelSortOptions, perform_pixel_sort},
        prepared::{
            PreparedSegmentSelectionMode,
            PreparedSegmentSortingMode,
            SegmentRandomizationMode,
            execute_axis_aligned_prepared_pixel_sort,
            prepare_pixel_sort,
            randomize_prepared_segments,
        },
    },
};

use crate::cancellation::CancellationToken;


pub enum WorkerRequest {
    OpenSourceImage {
        input_file_path: PathBuf,
    },

    #[allow(dead_code)]
    PerformImmediatePixelSorting {
        image: Arc<RgbaImage>,
        method: ImmediateSegmentSelectionMode,
        options: PixelSortOptions,
    },

    PerformPreparedPixelSorting {
        image: Arc<RgbaImage>,
        segment_selection_mode: PreparedSegmentSelectionMode,
        segment_randomization_mode: Option<SegmentRandomizationMode>,
        sorting_mode: PreparedSegmentSortingMode,
        sorting_direction: ImageSortingDirection,
    },

    ShowThresholdPreview {
        image: Arc<RgbaImage>,
        method: FeedbackSegmentSelectionMode,
        requested_at: Instant,
    },

    SaveImage {
        image: Arc<RgbaImage>,
        output_file_path: PathBuf,
    },
}

#[allow(clippy::enum_variant_names)]
pub enum WorkerResponse {
    OpenedSourceImage {
        file_path: PathBuf,
        image: RgbaImage,
    },

    FailedToOpenSourceImage {
        error: ImageLoadError,
    },

    ProcessedImage {
        image: RgbaImage,
    },

    ProcessedThresholdPreview {
        image: RgbaImage,
        requested_at: Instant,
    },

    SavedImage {
        output_file_path: PathBuf,
    },

    FailedToSaveImage {
        error: ImageSaveError,
    },
}

pub struct WorkerHandle {
    request_sender: Sender<WorkerRequest>,
    response_receiver: Receiver<WorkerResponse>,
    background_thread_cancellation_token: CancellationToken,
    background_thread_join_handle: JoinHandle<()>,
}

impl WorkerHandle {
    pub fn initialize() -> Self {
        let (req_sender, req_receiver) = crossbeam_channel::bounded::<WorkerRequest>(32);
        let (resp_sender, resp_receiver) = crossbeam_channel::bounded::<WorkerResponse>(32);

        let cancellation_token = CancellationToken::new();
        let cancellation_token_clone = cancellation_token.clone();

        let background_thread_join_handle = thread::spawn(move || {
            background_worker_loop(
                req_receiver,
                resp_sender,
                cancellation_token_clone,
            );
        });

        Self {
            request_sender: req_sender,
            response_receiver: resp_receiver,
            background_thread_cancellation_token: cancellation_token,
            background_thread_join_handle,
        }
    }

    pub fn sender(&self) -> &Sender<WorkerRequest> {
        &self.request_sender
    }

    pub fn receiver(&self) -> &Receiver<WorkerResponse> {
        &self.response_receiver
    }

    #[allow(dead_code)]
    pub fn stop_worker_and_join(self) {
        self.background_thread_cancellation_token.cancel();

        self.background_thread_join_handle
            .join()
            .expect("background worker thread has panicked");
    }
}

#[derive(Debug, Error)]
pub enum ImageLoadError {
    #[error("failed to open and/or read file")]
    FileReadError {
        #[source]
        error: io::Error,
    },

    #[error("failed to parse image (could be an unsupported format?)")]
    ImageParseError {
        #[source]
        error: image::ImageError,
    },
}

fn load_image_from_path(path: &Path) -> Result<RgbaImage, ImageLoadError> {
    let loaded_file_bytes =
        fs::read(path).map_err(|error| ImageLoadError::FileReadError { error })?;

    let parsed_image = image::load_from_memory(&loaded_file_bytes)
        .map_err(|error| ImageLoadError::ImageParseError { error })?;

    let image_as_rgba8 = parsed_image.to_rgba8();

    Ok(image_as_rgba8)
}

fn background_worker_loop(
    request_receiver: Receiver<WorkerRequest>,
    response_sender: Sender<WorkerResponse>,
    cancellation_token: CancellationToken,
) {
    loop {
        if cancellation_token.is_cancelled() {
            tracing::debug!("Cancellation token is set, exiting background worker.");
            break;
        }

        let request_result = request_receiver.recv_timeout(Duration::from_millis(50));
        let request = match request_result {
            Ok(request) => request,
            Err(error) => match error {
                RecvTimeoutError::Timeout => continue,
                RecvTimeoutError::Disconnected => {
                    tracing::error!(
                        "Background worker's request channel is empty and disconnected."
                    );
                    break;
                }
            },
        };

        match request {
            WorkerRequest::OpenSourceImage {
                input_file_path: file_path,
            } => {
                let loaded_image_result = load_image_from_path(&file_path);

                let response_result = match loaded_image_result {
                    Ok(image) => {
                        response_sender.send(WorkerResponse::OpenedSourceImage { image, file_path })
                    }
                    Err(error) => {
                        response_sender.send(WorkerResponse::FailedToOpenSourceImage { error })
                    }
                };

                if response_result.is_err() {
                    tracing::error!("Background worker's response channel is disconnected.");
                    break;
                }
            }
            WorkerRequest::PerformImmediatePixelSorting {
                image,
                method,
                options,
            } => {
                let image_copy = image.deref().to_owned();
                let sorted_image = perform_pixel_sort(image_copy, method, options);

                let response_result = response_sender.send(WorkerResponse::ProcessedImage {
                    image: sorted_image,
                });

                if response_result.is_err() {
                    tracing::error!("Background worker's response channel is disconnected.");
                    break;
                }
            }
            WorkerRequest::PerformPreparedPixelSorting {
                image,
                segment_selection_mode,
                segment_randomization_mode,
                sorting_mode,
                sorting_direction,
            } => {
                let image_copy = image.deref().to_owned();

                let prepared_sort = prepare_pixel_sort(
                    image_copy,
                    segment_selection_mode,
                    sorting_mode,
                    sorting_direction,
                );

                let prepared_sort =
                    if let Some(segment_randomization_mode) = segment_randomization_mode {
                        randomize_prepared_segments(prepared_sort, segment_randomization_mode)
                    } else {
                        prepared_sort
                    };

                // DEBUGONLY
                // println!("prepared: {prepared_sort:?}");

                let sorted_image = execute_axis_aligned_prepared_pixel_sort(prepared_sort);

                let response_result = response_sender.send(WorkerResponse::ProcessedImage {
                    image: sorted_image,
                });

                if response_result.is_err() {
                    tracing::error!("Background worker's response channel is disconnected.");
                    break;
                }
            }
            WorkerRequest::ShowThresholdPreview {
                image,
                method,
                requested_at,
            } => {
                let mut image_copy = image.deref().to_owned();

                mask_out_non_targeted_pixels(&mut image_copy, method, PIXEL_BLACK);

                let response_result =
                    response_sender.send(WorkerResponse::ProcessedThresholdPreview {
                        image: image_copy,
                        requested_at,
                    });

                if response_result.is_err() {
                    tracing::error!("Background worker's response channel is disconnected.");
                    break;
                }
            }
            WorkerRequest::SaveImage {
                image,
                output_file_path,
            } => {
                let save_result = save_image_as_png(
                    &DynamicImage::ImageRgba8(image.deref().to_owned()),
                    &output_file_path,
                    false,
                );

                let response_result = match save_result {
                    Ok(_) => response_sender.send(WorkerResponse::SavedImage { output_file_path }),
                    Err(error) => response_sender.send(WorkerResponse::FailedToSaveImage { error }),
                };

                if response_result.is_err() {
                    tracing::error!("Background worker's response channel is disconnected.");
                    break;
                }
            }
        }
    }
}
