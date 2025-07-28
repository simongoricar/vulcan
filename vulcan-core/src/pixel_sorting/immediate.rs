use std::cmp::Ordering;

use image::{Rgba, RgbaImage, flat::SampleLayout};
use rayon::prelude::*;

use crate::pixel_sorting::{
    ImageSortingDirection,
    PixelSegmentScannerState,
    PixelSegmentSortDirection,
    PixelWithContext,
    properties::{
        compute_rgba_hsl_hue,
        compute_rgba_hsl_saturation,
        compute_rgba_relative_luminance,
    },
    retrieve_rgba_pixel_from_flat_samples,
    sorting::sort_with_closure_and_reapply_pixel_segment,
};

pub enum SinglePassSegmentSelectionMode {
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
    // CannyEdges {
    //     /// The inclusive low end of the Canny edge detection threshold (`0.0..=1140.39`, see [`canny`][imageproc::edges::canny]).
    //     low: f32,
    //
    //     /// The inclusive high end of the Canny edge detection threshold (`0.0..=1140.39`, see [`canny`][imageproc::edges::canny]).
    //     high: f32,
    //
    //     /// Whether the first sortable pixel segment starts on the edge of the image, or at the first detected edge inside the image.
    //     /// Depends on the kind of effect you want; this will basically invert the segment ranges.
    //     segment_starts_on_image_edge: bool,
    // },
}



pub struct PixelSortOptions {
    pub direction: ImageSortingDirection,
}


/// Pixel sorts the given `image`.
///
/// TODO document
pub fn perform_pixel_sort(
    image: RgbaImage,
    method: SinglePassSegmentSelectionMode,
    options: PixelSortOptions,
) -> RgbaImage {
    match method {
        SinglePassSegmentSelectionMode::LuminanceRange { low, high } => {
            let relative_luminance_range = low..=high;

            perform_axis_aligned_generic_pixel_sort(
                image,
                options,
                |pixel: &Rgba<u8>| -> f32 {
                    compute_rgba_relative_luminance(pixel)
                },
                |pixel: &PixelWithContext<f32>| -> bool {
                    relative_luminance_range.contains(&pixel.context)
                },
            )
        }
        SinglePassSegmentSelectionMode::HueRange { low, high } => {
            let hue_range = low..=high;

            perform_axis_aligned_generic_pixel_sort(
                image,
                options,
                |pixel: &Rgba<u8>| -> f32 { compute_rgba_hsl_hue(pixel) },
                |pixel: &PixelWithContext<f32>| -> bool {
                    hue_range.contains(&pixel.context)
                },
            )
        }
        SinglePassSegmentSelectionMode::SaturationRange { low, high } => {
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
    }
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



// TODO document
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



// #[cfg(test)]
// mod test {
//     use super::*;
//
//     #[test]
//     fn gamma_to_linear_conversion_is_correct() {
//         todo!();
//     }
// }
