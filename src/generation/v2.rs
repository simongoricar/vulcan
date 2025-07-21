use std::ops::RangeInclusive;

use image::{Rgba, RgbaImage};
use rayon::prelude::*;

pub enum PixelSortMethod {
    LuminanceRange { low: f32, high: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SingleAxisDirection {
    Ascending,
    Descending,
}

pub enum PixelSortingDirection {
    Horizontal(SingleAxisDirection),
    Vertical(SingleAxisDirection),
}


pub struct PixelSortOptions {
    pub direction: PixelSortingDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowIterationState {
    OutsideSortableSegment,
    InsideSortableSegment { starting_column_index: u32 },
}


#[derive(Debug, Clone, PartialEq)]
enum StatefulRowIterationState {
    OutsideSortableSegment,
    InsideSortableSegment {
        starting_column_index: u32,
        collected_pixels: Vec<PixelWithRelativeLuminance>,
    },
}

/// See <https://en.wikipedia.org/wiki/Relative_luminance> for more information.
///
/// TODO This can be improved: ^2.2 does not fully match the gamma->linear conversion,
///      see the transfer function here: <https://en.wikipedia.org/wiki/SRGB>
///      and here <https://stackoverflow.com/questions/596216/formula-to-determine-perceived-brightness-of-rgb-color>.
#[inline(always)]
fn convert_gamma_encoded_srgb_to_linear(value: u8) -> u8 {
    let input_value_as_f32 = value as f32 / u8::MAX as f32;
    let output_value_as_f32 = input_value_as_f32.powf(2.2);

    (output_value_as_f32 * u8::MAX as f32) as u8
}

/// See <https://www.w3.org/WAI/GL/wiki/Relative_luminance> for more information.
///
/// Returns a value from zero to one.
fn compute_rgba_relative_luminance(pixel: &Rgba<u8>) -> f32 {
    let linear_r = convert_gamma_encoded_srgb_to_linear(pixel.0[0]);
    let linear_g = convert_gamma_encoded_srgb_to_linear(pixel.0[1]);
    let linear_b = convert_gamma_encoded_srgb_to_linear(pixel.0[2]);

    let relative_luminance_up_to_u8_range = 0.2126f32 * (linear_r as f32)
        + 0.7152f32 * (linear_g as f32)
        + 0.0722f32 * (linear_b as f32);

    relative_luminance_up_to_u8_range / (u8::MAX as f32)
}

pub fn perform_pixel_sort(
    image: RgbaImage,
    method: PixelSortMethod,
    options: PixelSortOptions,
) -> RgbaImage {
    match method {
        PixelSortMethod::LuminanceRange { low, high } => {
            perform_axis_aligned_luminance_range_pixel_sort(
                image, low, high, options,
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PixelWithRelativeLuminance {
    pixel: Rgba<u8>,
    relative_luminance: f32,
}

#[inline(always)]
fn retrieve_rgba_pixel_from_flat_samples(
    flat_slice: &[u8],
    pixel_index: usize,
    channel_stride: usize,
    num_channels: usize,
) -> Rgba<u8> {
    Rgba([
        flat_slice[pixel_index * channel_stride * num_channels],
        flat_slice[pixel_index * channel_stride * num_channels + channel_stride],
        flat_slice
            [pixel_index * channel_stride * num_channels + 2 * channel_stride],
        flat_slice
            [pixel_index * channel_stride * num_channels + 3 * channel_stride],
    ])
}

fn sort_and_reapply_pixel_segment(
    mut pixels: Vec<PixelWithRelativeLuminance>,
    sort_direction: SingleAxisDirection,
    target_flat_slice: &mut [u8],
    target_channel_stride: usize,
    target_num_channels: usize,
) {
    // Sort pixels.
    match sort_direction {
        SingleAxisDirection::Ascending => {
            pixels.sort_unstable_by(|first, second| {
                first
                    .relative_luminance
                    .total_cmp(&second.relative_luminance)
            });
        }
        SingleAxisDirection::Descending => {
            pixels.sort_unstable_by(|first, second| {
                second
                    .relative_luminance
                    .total_cmp(&first.relative_luminance)
            });
        }
    }

    // Reapply the sorted pixel segment back onto the image.
    for (pixel_index, pixel) in pixels.into_iter().enumerate() {
        let pixel_data = pixel.pixel.0;

        target_flat_slice
            [pixel_index * target_channel_stride * target_num_channels] =
            pixel_data[0];

        target_flat_slice[pixel_index
            * target_channel_stride
            * target_num_channels
            + target_channel_stride] = pixel_data[1];

        target_flat_slice[pixel_index
            * target_channel_stride
            * target_num_channels
            + 2 * target_channel_stride] = pixel_data[2];

        target_flat_slice[pixel_index
            * target_channel_stride
            * target_num_channels
            + 3 * target_channel_stride] = pixel_data[3];
    }
}


/// Does a horizontal, luminance range-based pixel sorting on a given row of the image.
///
/// Given `image_row_flat_samples`, an RGBA `u8` buffer of a single image row,
/// this function performs a horizontal pixel sort on sub-ranges in that row based on the
/// given `relative_luminance_range`, which controls *what pixels* should be sorted,
/// and `sort_direction`, which controls *in which direction* the pixels should be sorted.
///
/// Sorting is performed in-place on `image_row_flat_samples`, and in parallel (using `rayon`).
///
/// # Invariants
/// - `image_row_flat_samples` must be an RGBA8 buffer.
/// - `image_row_flat_samples` must point to a single row of the image.
/// - `relative_luminance_range` must not be outside of the range `0.0..=1.0`
///   (i.e. cannot start below zero end above one).
fn perform_horizontal_luminance_range_pixel_sort_on_image_row(
    image_row_flat_samples: &mut [u8],
    image_width: u32,
    channel_stride: usize,
    num_channels: usize,
    relative_luminance_range: RangeInclusive<f32>,
    sort_direction: SingleAxisDirection,
) {
    assert!(*relative_luminance_range.start() >= 0.0);
    assert!(*relative_luminance_range.end() <= 1.0);

    let mut current_state = StatefulRowIterationState::OutsideSortableSegment;

    #[allow(clippy::collapsible_else_if)]
    for column_index in 0..image_width {
        let column_index_usize = column_index as usize;

        let pixel = retrieve_rgba_pixel_from_flat_samples(
            image_row_flat_samples,
            column_index_usize,
            channel_stride,
            num_channels,
        );

        let relative_luminance = compute_rgba_relative_luminance(&pixel);

        if relative_luminance_range.contains(&relative_luminance) {
            if matches!(
                current_state,
                StatefulRowIterationState::OutsideSortableSegment
            ) {
                // Enter a new pixel sorting segment.
                current_state =
                    StatefulRowIterationState::InsideSortableSegment {
                        starting_column_index: column_index,
                        collected_pixels: vec![PixelWithRelativeLuminance {
                            pixel,
                            relative_luminance,
                        }],
                    }
            } else if let StatefulRowIterationState::InsideSortableSegment {
                starting_column_index,
                mut collected_pixels,
            } = current_state
            {
                collected_pixels.push(PixelWithRelativeLuminance {
                    pixel,
                    relative_luminance,
                });

                current_state =
                    StatefulRowIterationState::InsideSortableSegment {
                        starting_column_index,
                        collected_pixels,
                    };
            } else {
                unreachable!();
            };
        } else {
            if let StatefulRowIterationState::InsideSortableSegment {
                starting_column_index,
                mut collected_pixels,
            } = current_state
            {
                collected_pixels.push(PixelWithRelativeLuminance {
                    pixel,
                    relative_luminance,
                });

                let (_, realigned_row_slice) = image_row_flat_samples
                    .split_at_mut(
                        starting_column_index as usize
                            * channel_stride
                            * num_channels,
                    );

                sort_and_reapply_pixel_segment(
                    collected_pixels,
                    sort_direction,
                    realigned_row_slice,
                    channel_stride,
                    num_channels,
                );

                current_state =
                    StatefulRowIterationState::OutsideSortableSegment;
            }
        }
    }

    if let StatefulRowIterationState::InsideSortableSegment {
        starting_column_index,
        collected_pixels,
    } = current_state
    {
        let (_, realigned_row_slice) = image_row_flat_samples.split_at_mut(
            starting_column_index as usize * channel_stride * num_channels,
        );

        sort_and_reapply_pixel_segment(
            collected_pixels,
            sort_direction,
            realigned_row_slice,
            channel_stride,
            num_channels,
        );
    }
}


/// Does a horizontal, luminance range-based pixel sorting on the given RGBA `u8` image.
///
/// TODO document
pub fn perform_axis_aligned_luminance_range_pixel_sort(
    mut image: RgbaImage,
    threshold_low: f32,
    threshold_high: f32,
    options: PixelSortOptions,
) -> RgbaImage {
    let target_range = threshold_low..=threshold_high;

    match options.direction {
        PixelSortingDirection::Horizontal(horizontal_direction) => {
            let image_width = image.width();

            // For performance reasons, we'll operate directly on the underlying RGBA8 image buffer.
            let mut flat_samples = image.as_flat_samples_mut();

            // This is known to us, since we are expecting RGBA8.
            // Still, we'll use the values from the `layout` struct directly from here on.
            assert!(!flat_samples.has_aliased_samples());
            assert!(flat_samples.layout.channel_stride == 1);
            assert!(flat_samples.layout.channels == 4);

            let channel_stride = flat_samples.layout.channel_stride;
            let number_of_channels = flat_samples.layout.channels as usize;
            let height_stride = flat_samples.layout.height_stride;

            // The pixel sorting is performed here in parallel for each row of the image
            // using `rayon`'s parallel iterators.
            let parallel_per_row_iterator =
                flat_samples.as_mut_slice().par_chunks_mut(height_stride);

            parallel_per_row_iterator.for_each(|row| {
                perform_horizontal_luminance_range_pixel_sort_on_image_row(
                    row,
                    image_width,
                    channel_stride,
                    number_of_channels,
                    target_range.clone(),
                    horizontal_direction,
                );
            });
        }
        PixelSortingDirection::Vertical(vertical_direction) => {
            let image_height = image.height();

            let mut rotated_image = image::imageops::rotate90(&image);

            // For performance reasons, we'll operate directly on the underlying RGBA8 image buffer.
            // The rows of this buffer correspond to columns in the original image
            // (we just rotated our source image by 90 degrees and we'll do the inverse afterwards).
            let mut flat_samples = rotated_image.as_flat_samples_mut();

            // This is known to us, since we are expecting RGBA8.
            // Still, we'll use the values from the `layout` struct directly from here on.
            assert!(!flat_samples.has_aliased_samples());
            assert!(flat_samples.layout.channel_stride == 1);
            assert!(flat_samples.layout.channels == 4);

            let channel_stride = flat_samples.layout.channel_stride;
            let number_of_channels = flat_samples.layout.channels as usize;
            let height_stride = flat_samples.layout.height_stride;

            // The pixel sorting is performed here in parallel for each row of the image
            // using `rayon`'s parallel iterators.
            let parallel_per_row_iterator =
                flat_samples.as_mut_slice().par_chunks_mut(height_stride);

            parallel_per_row_iterator.for_each(|row| {
                perform_horizontal_luminance_range_pixel_sort_on_image_row(
                    row,
                    image_height,
                    channel_stride,
                    number_of_channels,
                    target_range.clone(),
                    vertical_direction,
                );
            });

            // PANIC SAFETY: This can only error if the image dimensions don't match.
            // However, this in impossible in our case, as 90 + 270 degrees = 360 degrees.
            image::imageops::rotate270_in(&rotated_image, &mut image)
                .expect("unexpected failure while inversing the image rotation");
        }
    }

    image
}
