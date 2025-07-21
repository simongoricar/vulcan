use std::num::NonZero;

use image::{Rgba, RgbaImage};
use rayon::prelude::*;

pub enum PixelSortMethod {
    LuminanceRange { low: u8, high: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelSortDirection {
    LeftToRight,
    RightToLeft,
    // TODO
    // TopToBottom,
    // BottomToTop,
}

pub struct PixelSortOptions {
    pub direction: PixelSortDirection,
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
            perform_luminance_range_pixel_sort(image, low, high, options)
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
    column_index: usize,
    channel_stride: usize,
    num_channels: usize,
) -> Rgba<u8> {
    Rgba([
        flat_slice[column_index * channel_stride * num_channels],
        flat_slice
            [column_index * channel_stride * num_channels + channel_stride],
        flat_slice
            [column_index * channel_stride * num_channels + 2 * channel_stride],
        flat_slice
            [column_index * channel_stride * num_channels + 3 * channel_stride],
    ])
}

fn sort_and_reapply_pixel_segment(
    mut pixels: Vec<PixelWithRelativeLuminance>,
    sort_direction: PixelSortDirection,
    target_flat_slice: &mut [u8],
    target_channel_stride: usize,
    target_num_channels: usize,
) {
    // Sort pixels.
    match sort_direction {
        PixelSortDirection::LeftToRight => {
            pixels.sort_unstable_by(|first, second| {
                first
                    .relative_luminance
                    .total_cmp(&second.relative_luminance)
            });
        }
        PixelSortDirection::RightToLeft => {
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



// TODO parallelize this!
pub fn perform_luminance_range_pixel_sort(
    mut image: RgbaImage,
    threshold_low: u8,
    threshold_high: u8,
    options: PixelSortOptions,
) -> RgbaImage {
    let target_range = (threshold_low as f32 / u8::MAX as f32)
        ..=(threshold_high as f32 / u8::MAX as f32);

    let image_width = image.width();

    let mut flat_samples = image.as_flat_samples_mut();
    assert!(!flat_samples.has_aliased_samples());
    assert!(flat_samples.layout.channel_stride == 1);

    let channel_stride = flat_samples.layout.channel_stride;
    let num_channels = flat_samples.layout.channels as usize;
    let width_of_row =
        flat_samples.layout.width as usize * num_channels * channel_stride;

    let parallel_per_row_iterator =
        flat_samples.as_mut_slice().par_chunks_mut(width_of_row);

    parallel_per_row_iterator.for_each(|row| {
        let mut current_state =
            StatefulRowIterationState::OutsideSortableSegment;
        
        #[allow(clippy::collapsible_else_if)]
        for column_index in 0..image_width {
            let column_index_usize = column_index as usize;

            let pixel = retrieve_rgba_pixel_from_flat_samples(
                row,
                column_index_usize,
                channel_stride,
                num_channels,
            );

            let relative_luminance = compute_rgba_relative_luminance(&pixel);

            if target_range.contains(&relative_luminance) {
                if matches!(current_state, StatefulRowIterationState::OutsideSortableSegment) {
                    // Enter a new pixel sorting segment.
                    current_state =
                        StatefulRowIterationState::InsideSortableSegment {
                            starting_column_index: column_index,
                            collected_pixels: vec![
                                PixelWithRelativeLuminance {
                                    pixel,
                                    relative_luminance,
                                },
                            ],
                        }
                } else if let StatefulRowIterationState::InsideSortableSegment { starting_column_index, mut collected_pixels } = current_state {
                    collected_pixels.push(PixelWithRelativeLuminance {
                        pixel,
                        relative_luminance,
                    });

                    current_state = StatefulRowIterationState::InsideSortableSegment { starting_column_index, collected_pixels };
                } else {
                    unreachable!();
                };
            } else {
                if let StatefulRowIterationState::InsideSortableSegment { starting_column_index, mut collected_pixels } = current_state {
                    collected_pixels.push(PixelWithRelativeLuminance {
                        pixel,
                        relative_luminance,
                    });

                    let (_, realigned_row_slice) = row.split_at_mut(
                        starting_column_index as usize
                            * channel_stride
                            * num_channels,
                    );

                    sort_and_reapply_pixel_segment(
                        collected_pixels,
                        options.direction,
                        realigned_row_slice,
                        channel_stride,
                        num_channels,
                    );

                    current_state = StatefulRowIterationState::OutsideSortableSegment;
                }
            }
        }

        if let StatefulRowIterationState::InsideSortableSegment {
            starting_column_index,
            collected_pixels,
        } = current_state
        {
            let (_, realigned_row_slice) = row.split_at_mut(
                starting_column_index as usize
                    * channel_stride
                    * num_channels,
            );

            sort_and_reapply_pixel_segment(
                collected_pixels,
                options.direction,
                realigned_row_slice,
                channel_stride,
                num_channels,
            );
        }
    });

    image
}
