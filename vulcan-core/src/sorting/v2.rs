use std::{
    cmp::Ordering,
    ops::{RangeInclusive, Rem},
};

use image::{Rgba, RgbaImage, flat::SampleLayout};
use num::Zero;
use rayon::prelude::*;

pub enum PixelSegmentSelectionMode {
    /// This mode creates pixel sorting segments that consist *only* of
    /// continuous pixels whose relative luminance[^relative-luminance]
    /// is between `low` and `high` (both inclusive).
    ///
    ///
    /// [^relative-luminance]: See [this Wikipedia article](https://en.wikipedia.org/wiki/Relative_luminance) for more information.
    LuminanceRange {
        /// The inclusive low end of the relative luminance range (`0.0..=1.0`).
        low: f32,

        /// The inclusive high end of the relative luminance range (`0.0..=1.0`).
        high: f32,
    },

    HueRange {
        /// The inclusive low end of the hue range (`0.0..360.0`).
        low: f32,

        /// The inclusive high end of the hue range (`0.0..360.0`).
        high: f32,
    },

    SaturationRange {
        /// The inclusive low end of the saturation range (`0.0..=1.0`).
        low: f32,

        /// The inclusive high end of the saturation range (`0.0..=1.0`).
        high: f32,
    },

    CannyEdges {
        /// The inclusive low end of the Canny edge detection threshold (`0.0..=1140.39`, see [`canny`][imageproc::edges::canny]).
        low: f32,

        /// The inclusive high end of the Canny edge detection threshold (`0.0..=1140.39`, see [`canny`][imageproc::edges::canny]).
        high: f32,

        /// Whether the first sortable pixel segment starts on the edge of the image, or at the first detected edge inside the image.
        /// Depends on the kind of effect you want; this will basically invert the segment ranges.
        segment_starts_on_image_edge: bool,
    },
}

/// Describes the direction in which a continuous segment of pixels is sorted;
/// either ascending or descending in regards to some underlying pixel property (set separately).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelSegmentSortDirection {
    Ascending,
    Descending,
}

/// The direction of pixel sorting.
pub enum ImageSortingDirection {
    /// Horizontal pixel sorting, either left-to-right or right-to-left.
    Horizontal(PixelSegmentSortDirection),

    /// Vertical pixel sorting, either top-to-bototm or bottom-to-top.
    Vertical(PixelSegmentSortDirection),
}


pub struct PixelSortOptions {
    pub direction: ImageSortingDirection,
}


/// A small internal enum containing pixel segment scanning state.
///
/// If in [`Self::OutsideSortableSegment`], no action is taken.
///
/// If in [`Self::InsideSortableSegment`], the starting index of the segment is tracked
/// as well as all the pixels that are in that segment so far (alongside with their
/// properties we'll use for sorting).
///
/// When exiting a sortable segment (e.g. when the next pixel falls out of the target
/// relative luminance range), we take the collected pixels and sort them,
/// then enter [`Self::OutsideSortableSegment`]. So, in a sense, this is a
/// tiny finite automata with state.
#[derive(Debug, Clone, PartialEq)]
enum PixelSegmentScannerState<P> {
    /// Represents a state in which we're not currently "in" any pixel sorting segment.
    OutsideSortableSegment,

    /// Represents a state in which we're currently "in" a new pixel sorting segment.
    /// We'll keep adding pixels into `collected_pixels` in this state as long as we are
    /// in one contiguous segment. After we're done, we'll take `collected_pixels`, sort them,
    /// and reapply them onto the image.
    CollectingSortableSegment {
        /// The starting pixel index of the segment, relative to our row or column of the image.
        segment_start_index: u32,

        /// The pixels in our sortable segment so far, alongside their precomputed properties.
        collected_pixels: Vec<P>,
    },
}

/// Converts a gamma-encoded `u8` sRGB value to a linear `u8` sRGB value.
///
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

/// Computes the relative luminance[^relative-luminance] of an RGBA pixel,
/// as an `f32` in the range `0.0..=1.0`.
///
///
/// [^relative-luminance]: See <https://www.w3.org/WAI/GL/wiki/Relative_luminance> for more information.
fn compute_rgba_relative_luminance(pixel: &Rgba<u8>) -> f32 {
    let linear_r = convert_gamma_encoded_srgb_to_linear(pixel.0[0]);
    let linear_g = convert_gamma_encoded_srgb_to_linear(pixel.0[1]);
    let linear_b = convert_gamma_encoded_srgb_to_linear(pixel.0[2]);

    let relative_luminance_up_to_u8_range = 0.2126f32 * (linear_r as f32)
        + 0.7152f32 * (linear_g as f32)
        + 0.0722f32 * (linear_b as f32);

    relative_luminance_up_to_u8_range / (u8::MAX as f32)
}

fn compute_rgba_hsl_hue(pixel: &Rgba<u8>) -> f32 {
    let linear_r = convert_gamma_encoded_srgb_to_linear(pixel.0[0]);
    let linear_g = convert_gamma_encoded_srgb_to_linear(pixel.0[1]);
    let linear_b = convert_gamma_encoded_srgb_to_linear(pixel.0[2]);

    let normalized_r = (linear_r as f32) / (u8::MAX as f32);
    let normalized_g = (linear_g as f32) / (u8::MAX as f32);
    let normalized_b = (linear_b as f32) / (u8::MAX as f32);

    let max_value = normalized_r.max(normalized_g).max(normalized_b);
    let min_value = normalized_r.min(normalized_g).min(normalized_b);

    let chroma = max_value - min_value;

    let hue_prime = if chroma.is_zero() {
        0f32
    } else if max_value == normalized_r {
        ((normalized_g - normalized_b) / chroma).rem(6f32)
    } else if max_value == normalized_g {
        ((normalized_b - normalized_r) / chroma) + 2f32
    } else if max_value == normalized_b {
        ((normalized_r - normalized_g) / chroma) + 4f32
    } else {
        unreachable!();
    };

    let hue = hue_prime * 60f32;

    hue
}

fn compute_rgba_hsl_lightness(pixel: &Rgba<u8>) -> f32 {
    let linear_r = convert_gamma_encoded_srgb_to_linear(pixel.0[0]);
    let linear_g = convert_gamma_encoded_srgb_to_linear(pixel.0[1]);
    let linear_b = convert_gamma_encoded_srgb_to_linear(pixel.0[2]);

    let normalized_r = (linear_r as f32) / (u8::MAX as f32);
    let normalized_g = (linear_g as f32) / (u8::MAX as f32);
    let normalized_b = (linear_b as f32) / (u8::MAX as f32);

    let max_value = normalized_r.max(normalized_g).max(normalized_b);
    let min_value = normalized_r.min(normalized_g).min(normalized_b);

    let lightness = (max_value + min_value) / 2f32;

    lightness
}

fn compute_rgba_hsl_saturation(pixel: &Rgba<u8>) -> f32 {
    let linear_r = convert_gamma_encoded_srgb_to_linear(pixel.0[0]);
    let linear_g = convert_gamma_encoded_srgb_to_linear(pixel.0[1]);
    let linear_b = convert_gamma_encoded_srgb_to_linear(pixel.0[2]);

    let normalized_r = (linear_r as f32) / (u8::MAX as f32);
    let normalized_g = (linear_g as f32) / (u8::MAX as f32);
    let normalized_b = (linear_b as f32) / (u8::MAX as f32);

    let max_value = normalized_r.max(normalized_g).max(normalized_b);
    let min_value = normalized_r.min(normalized_g).min(normalized_b);

    let lightness = (max_value + min_value) / 2f32;

    // See <https://en.wikipedia.org/wiki/HSL_and_HSV#From_RGB>
    let saturation = if lightness == 0f32 || lightness == 1f32 {
        0f32
    } else {
        (max_value - lightness) / lightness.min(1f32 - lightness)
    };

    saturation
}

/// Pixel sorts the given `image`.
///
/// TODO document
pub fn perform_pixel_sort(
    image: RgbaImage,
    method: PixelSegmentSelectionMode,
    options: PixelSortOptions,
) -> RgbaImage {
    match method {
        PixelSegmentSelectionMode::LuminanceRange { low, high } => {
            let relative_luminance_range = low..=high;

            perform_axis_aligned_generic_pixel_sort(
                image,
                options,
                |pixel: &Rgba<u8>| -> f32 {
                    compute_rgba_relative_luminance(pixel)
                },
                |context: &PixelWithContext<f32>| -> bool {
                    relative_luminance_range.contains(&context.context)
                },
            )
        }
        PixelSegmentSelectionMode::HueRange { low, high } => {
            let hue_range = low..=high;

            perform_axis_aligned_generic_pixel_sort(
                image,
                options,
                |pixel: &Rgba<u8>| -> f32 { compute_rgba_hsl_hue(pixel) },
                |context: &PixelWithContext<f32>| -> bool {
                    hue_range.contains(&context.context)
                },
            )
        }
        PixelSegmentSelectionMode::SaturationRange { low, high } => {
            let saturation_range = low..=high;

            perform_axis_aligned_generic_pixel_sort(
                image,
                options,
                |pixel: &Rgba<u8>| -> f32 { compute_rgba_hsl_saturation(pixel) },
                |context: &PixelWithContext<f32>| -> bool {
                    saturation_range.contains(&context.context)
                },
            )
        }
        PixelSegmentSelectionMode::CannyEdges {
            low,
            high,
            segment_starts_on_image_edge,
        } => {
            todo!();
            // TODO edge-detected image method
        }
    }
}


/// An internal struct that carries contextual information (e.g. relative luminance)
/// alongside the actual [`Rgba`]`<`[`u8`]`>` pixel value.
#[derive(Debug, Clone, PartialEq)]
pub struct PixelWithContext<C> {
    pub pixel: Rgba<u8>,
    pub context: C,
}

impl<C> AsRef<Rgba<u8>> for PixelWithContext<C> {
    fn as_ref(&self) -> &Rgba<u8> {
        &self.pixel
    }
}


/// Returns data about a single RGBA pixel ([`Rgba`]`<`[`u8`]`>`) at some specific pixel index
/// in the given `flat_slice` of the image.
///
/// # Invariants
/// - The `flat_slice` must be the flat sample buffer of an RGBA8 image.
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



/// Given a `Vec` of pixels and a contiguous RGBA8 image buffer,
/// this function will copy the pixels one after another onto that buffer,
/// overwriting the previous pixel values.
///
/// # Panics
/// The length of `target_image_contiguous_flat_buffer` must be precisely large
/// enough to fit all the `source_pixels`; the function will otherwise panic.
fn copy_pixel_segment_onto_image<P>(
    source_pixels: Vec<P>,
    target_contiguous_flat_buffer: &mut [u8],
    target_layout: SampleLayout,
) where
    P: AsRef<Rgba<u8>>,
{
    assert!(
        source_pixels.len()
            * target_layout.channel_stride
            * target_layout.channels as usize
            == target_contiguous_flat_buffer.len()
    );

    // Reapply the sorted pixel segment back onto the image.
    let channel_stride = target_layout.channel_stride;
    let number_of_channels = target_layout.channels as usize;

    for (pixel_index, pixel) in source_pixels.into_iter().enumerate() {
        let pixel_data = pixel.as_ref().0;

        target_contiguous_flat_buffer
            [pixel_index * channel_stride * number_of_channels] = pixel_data[0];

        target_contiguous_flat_buffer[pixel_index
            * channel_stride
            * number_of_channels
            + channel_stride] = pixel_data[1];

        target_contiguous_flat_buffer[pixel_index
            * channel_stride
            * number_of_channels
            + 2 * channel_stride] = pixel_data[2];

        target_contiguous_flat_buffer[pixel_index
            * channel_stride
            * number_of_channels
            + 3 * channel_stride] = pixel_data[3];
    }
}



/// Sorts the given contextualized `pixels` using the sorting closure,
/// then copies the sorted pixels onto the target image, provided as a flat RGBA8 buffer
/// (`target_image_contiguous_flat_buffer`).
///
/// # Panics
/// The length of `target_image_contiguous_flat_buffer` must be precisely large
/// enough to fit all the `source_pixels`; the function will otherwise panic.
fn sort_with_closure_and_reapply_pixel_segment<C, S>(
    mut pixels: Vec<PixelWithContext<C>>,
    target_image_contiguous_flat_buffer: &mut [u8],
    target_image_layout: SampleLayout,
    segment_sorting_closure: S,
) where
    S: FnOnce(&mut [PixelWithContext<C>]),
{
    segment_sorting_closure(&mut pixels);

    // Reapply the sorted pixel segment back onto the image at the correct position.
    copy_pixel_segment_onto_image(
        pixels,
        target_image_contiguous_flat_buffer,
        target_image_layout,
    );
}



/// Sorts the given "contextualized" `pixels` ([`Vec`]`<`[`PixelWithContext`]`<C>>`) in the provided direction, then
/// copies the sorted pixels onto the target image, provided as a flat RGBA8 buffer
/// (`target_image_contiguous_flat_buffer`). This is a specialized version of
/// [`sort_with_closure_and_reapply_pixel_segment`], for cases where the pixel context
/// is a number, e.g. an `f32`.
///
/// # Panics
/// The length of `target_image_contiguous_flat_buffer` must be precisely large
/// enough to fit all the `source_pixels`; the function will otherwise panic.
fn sort_with_numeric_context_and_reapply_pixel_segment<C>(
    mut pixels: Vec<PixelWithContext<C>>,
    sort_direction: PixelSegmentSortDirection,
    target_image_contiguous_flat_buffer: &mut [u8],
    target_image_layout: SampleLayout,
) where
    C: num::Num + Copy + PartialOrd,
{
    assert!(
        pixels.len()
            * target_image_layout.channel_stride
            * target_image_layout.channels as usize
            == target_image_contiguous_flat_buffer.len()
    );

    // Sort pixels.
    match sort_direction {
        PixelSegmentSortDirection::Ascending => {
            pixels.sort_unstable_by(|first, second| {
                first
                    .context
                    .partial_cmp(&second.context)
                    .unwrap_or(Ordering::Equal)
            });
        }
        PixelSegmentSortDirection::Descending => {
            pixels.sort_unstable_by(|first, second| {
                second
                    .context
                    .partial_cmp(&first.context)
                    .unwrap_or(Ordering::Equal)
            });
        }
    }

    // Reapply the sorted pixel segment back onto the image at the correct position.
    copy_pixel_segment_onto_image(
        pixels,
        target_image_contiguous_flat_buffer,
        target_image_layout,
    );
}



/// Performs horizontal pixel sorting on a given row of the image, using the provided
/// closures to compute pixel context, assign segment membership, and sort the final pixel segments.
/// This allows the caller to customize the sorting quite precisely.
///
/// Sorting is performed in-place on `image_contiguous_flat_buffer`, and in parallel (using `rayon`).
///
/// # Invariants
/// - `image_contiguous_flat_buffer` must be an RGBA8 buffer.
/// - `image_contiguous_flat_buffer` must point to a single row of the image.
/// - `relative_luminance_range` must not be outside of the range `0.0..=1.0`
///   (i.e. cannot start below zero end above one).
fn perform_generic_pixel_sort_on_image_row<
    C,
    ContextClosure,
    MembershipClosure,
    SortingClosure,
>(
    // Should point to a single row or column of the image as a flat RGBA8 sample buffer.
    image_contiguous_flat_buffer: &mut [u8],
    image_layout: SampleLayout,
    pixel_context_computation_closure: ContextClosure,
    segment_membership_closure: MembershipClosure,
    mut segment_sorting_closure: SortingClosure,
) where
    ContextClosure: Fn(&Rgba<u8>) -> C,
    MembershipClosure: Fn(&PixelWithContext<C>) -> bool,
    SortingClosure: FnMut(&mut [PixelWithContext<C>]),
{
    let mut current_state: PixelSegmentScannerState<PixelWithContext<C>> =
        PixelSegmentScannerState::OutsideSortableSegment;

    let image_channel_stride = image_layout.channel_stride;
    let image_number_of_channels = image_layout.channels as usize;

    #[allow(clippy::collapsible_else_if)]
    for column_index in 0..image_layout.width {
        let column_index_usize = column_index as usize;

        let pixel = retrieve_rgba_pixel_from_flat_samples(
            image_contiguous_flat_buffer,
            column_index_usize,
            image_channel_stride,
            image_number_of_channels,
        );

        let pixel_property = pixel_context_computation_closure(&pixel);
        let pixel_with_property = PixelWithContext {
            pixel,
            context: pixel_property,
        };

        let belongs_inside_sorted_segment =
            segment_membership_closure(&pixel_with_property);

        if belongs_inside_sorted_segment {
            match current_state {
                PixelSegmentScannerState::OutsideSortableSegment => {
                    // Enter a new pixel sorting segment.
                    current_state =
                        PixelSegmentScannerState::CollectingSortableSegment {
                            segment_start_index: column_index,
                            collected_pixels: vec![pixel_with_property],
                        };
                }
                PixelSegmentScannerState::CollectingSortableSegment {
                    segment_start_index,
                    mut collected_pixels,
                } => {
                    collected_pixels.push(pixel_with_property);

                    current_state =
                        PixelSegmentScannerState::CollectingSortableSegment {
                            segment_start_index,
                            collected_pixels,
                        };
                }
            }
        } else {
            match current_state {
                PixelSegmentScannerState::OutsideSortableSegment => {
                    current_state =
                        PixelSegmentScannerState::OutsideSortableSegment;
                }
                PixelSegmentScannerState::CollectingSortableSegment {
                    segment_start_index,
                    mut collected_pixels,
                } => {
                    collected_pixels.push(pixel_with_property);

                    let (_, realigned_row_slice) = image_contiguous_flat_buffer
                        .split_at_mut(
                            segment_start_index as usize
                                * image_channel_stride
                                * image_number_of_channels,
                        );

                    let (clipped_segment_slice, _) = realigned_row_slice
                        .split_at_mut(
                            collected_pixels.len()
                                * image_channel_stride
                                * image_number_of_channels,
                        );

                    sort_with_closure_and_reapply_pixel_segment(
                        collected_pixels,
                        clipped_segment_slice,
                        image_layout,
                        &mut segment_sorting_closure,
                    );

                    current_state =
                        PixelSegmentScannerState::OutsideSortableSegment;
                }
            }
        }
    }

    // If the last pixel was also inside a sortable segment,
    // we conclude that segment here and perform one final sorting.
    if let PixelSegmentScannerState::CollectingSortableSegment {
        segment_start_index,
        collected_pixels,
    } = current_state
    {
        let (_, realigned_row_slice) = image_contiguous_flat_buffer
            .split_at_mut(
                segment_start_index as usize
                    * image_channel_stride
                    * image_number_of_channels,
            );

        let (clipped_segment_slice, _) = realigned_row_slice.split_at_mut(
            collected_pixels.len()
                * image_channel_stride
                * image_number_of_channels,
        );

        sort_with_closure_and_reapply_pixel_segment(
            collected_pixels,
            clipped_segment_slice,
            image_layout,
            &mut segment_sorting_closure,
        );
    }
}


/// Given the mutable slice `pixels_in_segment`, this function
/// sorts the pixels in-place by sorting their numeric context
/// (generic `C`; must be a number) in the provided `sorting_direction`.
fn sort_array_of_numeric_contextual_pixels_by_direction<C>(
    pixels_in_segment: &mut [PixelWithContext<C>],
    sorting_direction: PixelSegmentSortDirection,
) where
    C: num::Num + Copy + PartialOrd,
{
    match sorting_direction {
        PixelSegmentSortDirection::Ascending => {
            pixels_in_segment.sort_unstable_by(|first, second| {
                first
                    .context
                    .partial_cmp(&second.context)
                    .unwrap_or(Ordering::Equal)
            });
        }
        PixelSegmentSortDirection::Descending => {
            pixels_in_segment.sort_unstable_by(|first, second| {
                second
                    .context
                    .partial_cmp(&first.context)
                    .unwrap_or(Ordering::Equal)
            });
        }
    }
}


// TODO make generic and document
fn perform_axis_aligned_generic_pixel_sort<
    PixelProperty,
    PropertyClosure,
    MembershipClosure,
>(
    mut image: RgbaImage,
    options: PixelSortOptions,
    pixel_context_computation_closure: PropertyClosure,
    segment_membership_closure: MembershipClosure,
) -> RgbaImage
where
    PixelProperty: num::Num + Copy + PartialOrd,
    PropertyClosure: Fn(&Rgba<u8>) -> PixelProperty + Sync + Send,
    MembershipClosure:
        Fn(&PixelWithContext<PixelProperty>) -> bool + Sync + Send,
{
    match options.direction {
        ImageSortingDirection::Horizontal(horizontal_direction) => {
            // For performance reasons, we'll operate directly on the underlying RGBA8 image buffer.
            let mut flat_samples = image.as_flat_samples_mut();

            // This is known to us, since we are expecting RGBA8.
            // Still, we'll use the values from the `layout` struct directly from here on.
            assert!(!flat_samples.has_aliased_samples());
            assert!(flat_samples.layout.channel_stride == 1);
            assert!(flat_samples.layout.channels == 4);

            let image_layout = flat_samples.layout;

            // The pixel sorting is performed here in parallel for each row of the image
            // using `rayon`'s parallel iterators.
            let parallel_per_row_iterator = flat_samples
                .as_mut_slice()
                .par_chunks_mut(image_layout.height_stride);

            parallel_per_row_iterator.for_each(|row_buffer| {
                perform_generic_pixel_sort_on_image_row(
                    row_buffer,
                    image_layout,
                    &pixel_context_computation_closure,
                    &segment_membership_closure,
                    |pixel_segment| {
                        sort_array_of_numeric_contextual_pixels_by_direction(
                            pixel_segment,
                            horizontal_direction,
                        );
                    },
                );
            });
        }
        ImageSortingDirection::Vertical(vertical_direction) => {
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

            let image_layout = flat_samples.layout;

            // The pixel sorting is performed here in parallel for each row of the image
            // using `rayon`'s parallel iterators.
            let parallel_per_row_iterator = flat_samples
                .as_mut_slice()
                .par_chunks_mut(image_layout.height_stride);

            parallel_per_row_iterator.for_each(|row_buffer| {
                perform_generic_pixel_sort_on_image_row(
                    row_buffer,
                    image_layout,
                    &pixel_context_computation_closure,
                    &segment_membership_closure,
                    |pixel_segment| {
                        sort_array_of_numeric_contextual_pixels_by_direction(
                            pixel_segment,
                            vertical_direction,
                        );
                    },
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
