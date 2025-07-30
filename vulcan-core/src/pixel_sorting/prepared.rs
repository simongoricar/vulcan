use std::fmt::Debug;

use image::{DynamicImage, GrayImage, Rgba, RgbaImage, flat::SampleLayout};
use rand::prelude::Distribution;
use rand_distr::{Normal, Uniform};
use rayon::prelude::{IndexedParallelIterator, ParallelIterator, ParallelSlice, ParallelSliceMut};

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
    retrieve_starting_rgba_pixel_from_flat_samples,
    sorting::sort_with_numeric_context_and_reapply_pixel_segment,
};

pub enum PreparedSegmentSortingMode {
    Luminance,
    Hue,
    Saturation,
}

pub enum PreparedSegmentSelectionMode {
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


enum PreparedPixelSortImage {
    PreparedHorizontal {
        /// The image to be sorted.
        image: RgbaImage,

        /// The horizontal direction in which the selected underlying pixel property will be sorted.
        direction: PixelSegmentSortDirection,
    },

    PreparedVertical {
        /// This is an already 90-degree-rotated image. After sorting, rotate by 270 degrees to obtain the original image orientation.
        rotated_image: RgbaImage,

        /// The vertical direction in which the selected underlying pixel property will be sorted.
        direction: PixelSegmentSortDirection,
    },
}

impl PreparedPixelSortImage {
    pub fn width(&self) -> usize {
        match self {
            Self::PreparedHorizontal { image, .. } => image.width() as usize,
            Self::PreparedVertical { rotated_image, .. } => rotated_image.height() as usize,
        }
    }

    pub fn height(&self) -> usize {
        match self {
            Self::PreparedHorizontal { image, .. } => image.height() as usize,
            Self::PreparedVertical { rotated_image, .. } => rotated_image.width() as usize,
        }
    }
}


/// This represents a prepared pixel sort where the sorting property is a number
/// (this allows us to set up sorting functions more simply).
pub struct PreparedPixelSort<SortingContext>
where
    SortingContext: Send + num::Num + Copy + PartialOrd,
{
    image: PreparedPixelSortImage,

    /// These are the custom sorting contexts, presented in row-major order.
    prepared_row_data: Vec<PreparedPixelSortRow<SortingContext>>,
}

impl<SortingContext> Debug for PreparedPixelSort<SortingContext>
where
    SortingContext: Send + num::Num + Copy + PartialOrd,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PreparedPixelSort{{{}x{}; segments-per-row: [",
            self.image.width(),
            self.image.height()
        )?;

        for (row_index, row) in self.prepared_row_data.iter().enumerate() {
            write!(f, "{}", row.sorting_contexts_for_row.len())?;

            if row_index < (self.prepared_row_data.len() - 1) {
                write!(f, ",")?;
            }
        }

        write!(f, "]}}")
    }
}

/// This represents a single continous segment of pixels that is to be sorted.
struct PreparedPixelSortSegment<SortingContext>
where
    SortingContext: Send,
{
    /// The image column index where this segment begins.
    start_column_index: usize,

    /// Contains the pre-computed sorting context for each pixel.
    /// The first element of this vector corresponds to the sorting context for the pixel
    /// at column `start_column_index`, the second one to the pixel at column `start_column_index + 1`, etc.
    ///
    /// The length of this vector is the length of the pixel sorting segment.
    pixel_sorting_contexts: Vec<SortingContext>,
}

/// This represents a single row of prepared pixel sorts.
pub struct PreparedPixelSortRow<SortingContext>
where
    SortingContext: Send,
{
    sorting_contexts_for_row: Vec<PreparedPixelSortSegment<SortingContext>>,
}


fn prepare_horizontal_generic_pixel_sort_for_image_row<
    MembershipContext,
    SortingContext,
    MembershipContextClosure,
    SegmentMembershipClosure,
    SortingContextClosure,
>(
    image_row_contiguous_flat_buffer: &[u8],
    image_layout: SampleLayout,
    segment_membership_context_computation_closure: MembershipContextClosure,
    segment_membership_closure: SegmentMembershipClosure,
    sorting_context_computation_closure: SortingContextClosure,
) -> PreparedPixelSortRow<SortingContext>
where
    SortingContext: Send,
    MembershipContextClosure: Fn(&Rgba<u8>) -> MembershipContext,
    SegmentMembershipClosure: Fn(&PixelWithContext<MembershipContext>) -> bool,
    SortingContextClosure: Fn(&PixelWithContext<MembershipContext>) -> SortingContext,
{
    let mut sorting_contexts_for_row: Vec<PreparedPixelSortSegment<SortingContext>> = Vec::new();

    let mut current_state: PixelSegmentScannerState<SortingContext> =
        PixelSegmentScannerState::OutsideSortableSegment;

    let image_width = image_layout.width;
    let image_channel_stride = image_layout.channel_stride;
    let image_number_of_channels = image_layout.channels as usize;

    for column_index in 0..image_width {
        let column_index_usize = column_index as usize;

        let pixel = retrieve_rgba_pixel_from_flat_samples(
            image_row_contiguous_flat_buffer,
            column_index_usize,
            image_channel_stride,
            image_number_of_channels,
        );

        let pixel_membership_context = segment_membership_context_computation_closure(&pixel);
        let pixel_with_context = PixelWithContext::new(pixel, pixel_membership_context);

        let belongs_inside_sorted_segment = segment_membership_closure(&pixel_with_context);

        if belongs_inside_sorted_segment {
            match current_state {
                // Enter a new pixel sorting segment.
                PixelSegmentScannerState::OutsideSortableSegment => {
                    let current_pixel_sorting_context =
                        sorting_context_computation_closure(&pixel_with_context);

                    current_state = PixelSegmentScannerState::CollectingSortableSegment {
                        segment_start_index: column_index,
                        collected_pixels: vec![current_pixel_sorting_context],
                    };
                }
                // Add the pixel to the current sorting segment.
                PixelSegmentScannerState::CollectingSortableSegment {
                    segment_start_index,
                    mut collected_pixels,
                } => {
                    let current_pixel_sorting_context =
                        sorting_context_computation_closure(&pixel_with_context);

                    collected_pixels.push(current_pixel_sorting_context);

                    current_state = PixelSegmentScannerState::CollectingSortableSegment {
                        segment_start_index,
                        collected_pixels,
                    };
                }
            }
        } else {
            match current_state {
                // Maintain the state.
                PixelSegmentScannerState::OutsideSortableSegment => {
                    current_state = PixelSegmentScannerState::OutsideSortableSegment;
                }
                // Exit the pixel sorting segment.
                PixelSegmentScannerState::CollectingSortableSegment {
                    segment_start_index,
                    mut collected_pixels,
                } => {
                    let current_pixel_sorting_context =
                        sorting_context_computation_closure(&pixel_with_context);

                    collected_pixels.push(current_pixel_sorting_context);

                    sorting_contexts_for_row.push(PreparedPixelSortSegment {
                        start_column_index: segment_start_index as usize,
                        pixel_sorting_contexts: collected_pixels,
                    });

                    current_state = PixelSegmentScannerState::OutsideSortableSegment;
                }
            }
        }
    }

    // If the last pixel was also inside a sortable segment,
    // we conclude that segment here.
    if let PixelSegmentScannerState::CollectingSortableSegment {
        segment_start_index,
        collected_pixels,
    } = current_state
    {
        sorting_contexts_for_row.push(PreparedPixelSortSegment {
            start_column_index: segment_start_index as usize,
            pixel_sorting_contexts: collected_pixels,
        });
    }

    PreparedPixelSortRow {
        sorting_contexts_for_row,
    }
}

fn prepare_axis_aligned_numeric_pixel_sort<
    MembershipContext,
    SortingContext,
    MembershipContextClosure,
    SegmentMembershipClosure,
    SortingContextClosure,
>(
    image: RgbaImage,
    direction: ImageSortingDirection,
    segment_membership_context_computation_closure: MembershipContextClosure,
    segment_membership_closure: SegmentMembershipClosure,
    sorting_context_computation_closure: SortingContextClosure,
) -> PreparedPixelSort<SortingContext>
where
    SortingContext: Send + num::Num + Copy + PartialOrd,
    MembershipContextClosure: Fn(&Rgba<u8>) -> MembershipContext + Send + Sync,
    SegmentMembershipClosure: Fn(&PixelWithContext<MembershipContext>) -> bool + Send + Sync,
    SortingContextClosure: Fn(&PixelWithContext<MembershipContext>) -> SortingContext + Send + Sync,
{
    match direction {
        ImageSortingDirection::Horizontal(pixel_segment_sort_direction) => {
            // For performance reasons, we'll operate directly on the underlying RGBA8 image buffer.
            let flat_samples = image.as_flat_samples();

            // This is known to us, since we are expecting RGBA8.
            // Still, we'll use the values from the `layout` struct directly from here on.
            assert!(!flat_samples.has_aliased_samples());
            assert!(flat_samples.layout.channel_stride == 1);
            assert!(flat_samples.layout.channels == 4);

            let image_layout = flat_samples.layout;

            // The segments are computed here in parallel for each row of the image
            // using `rayon`'s parallel iterators.
            let parallel_per_row_iterator = flat_samples
                .as_slice()
                .par_chunks(image_layout.height_stride);

            let prepared_rows = parallel_per_row_iterator
                .map(|row_buffer| {
                    prepare_horizontal_generic_pixel_sort_for_image_row(
                        row_buffer,
                        image_layout,
                        &segment_membership_context_computation_closure,
                        &segment_membership_closure,
                        &sorting_context_computation_closure,
                    )
                })
                .collect::<Vec<_>>();

            assert!(prepared_rows.len() == image_layout.height as usize);
            let mut prepared_row_data: Vec<PreparedPixelSortRow<SortingContext>> =
                Vec::with_capacity(prepared_rows.len());

            for row_data in prepared_rows {
                prepared_row_data.push(row_data);
            }

            PreparedPixelSort {
                image: PreparedPixelSortImage::PreparedHorizontal {
                    image,
                    direction: pixel_segment_sort_direction,
                },
                prepared_row_data,
            }
        }
        ImageSortingDirection::Vertical(pixel_segment_sort_direction) => {
            let rotated_image = image::imageops::rotate90(&image);

            // For performance reasons, we'll operate directly on the underlying RGBA8 image buffer.
            let flat_samples = rotated_image.as_flat_samples();

            // This is known to us, since we are expecting RGBA8.
            // Still, we'll use the values from the `layout` struct directly from here on.
            assert!(!flat_samples.has_aliased_samples());
            assert!(flat_samples.layout.channel_stride == 1);
            assert!(flat_samples.layout.channels == 4);

            let rotated_image_layout = flat_samples.layout;

            // The segments are computed here in parallel for each row of the image
            // using `rayon`'s parallel iterators.
            let parallel_per_row_iterator = flat_samples
                .as_slice()
                .par_chunks(rotated_image_layout.height_stride);

            let prepared_rows = parallel_per_row_iterator
                .map(|row_buffer| {
                    prepare_horizontal_generic_pixel_sort_for_image_row(
                        row_buffer,
                        rotated_image_layout,
                        &segment_membership_context_computation_closure,
                        &segment_membership_closure,
                        &sorting_context_computation_closure,
                    )
                })
                .collect::<Vec<_>>();

            assert!(prepared_rows.len() == rotated_image_layout.height as usize);
            let mut prepared_row_data: Vec<PreparedPixelSortRow<SortingContext>> =
                Vec::with_capacity(prepared_rows.len());

            for row_data in prepared_rows {
                prepared_row_data.push(row_data);
            }

            PreparedPixelSort {
                image: PreparedPixelSortImage::PreparedVertical {
                    rotated_image,
                    direction: pixel_segment_sort_direction,
                },
                prepared_row_data,
            }
        }
    }
}


fn prepare_segments_using_detected_edges_for_single_row<SortingContext, SortingContextClosure>(
    target_image_row_contiguous_flat_buffer: &[u8],
    target_image_layout: SampleLayout,
    edge_image_row_contiguous_flat_buffer: &[u8],
    edge_image_layout: SampleLayout,
    initial_segment_starts_on_left_edge: bool,
    sorting_context_computation_closure: SortingContextClosure,
) -> PreparedPixelSortRow<SortingContext>
where
    SortingContext: Send + num::Num + Copy + PartialOrd,
    SortingContextClosure: Fn(&Rgba<u8>) -> SortingContext + Send + Sync,
{
    assert_eq!(edge_image_layout.width_stride, 1);
    assert_eq!(
        target_image_row_contiguous_flat_buffer.len() / target_image_layout.width_stride,
        edge_image_row_contiguous_flat_buffer.len() / edge_image_layout.width_stride
    );
    assert_eq!(
        edge_image_row_contiguous_flat_buffer.len() / edge_image_layout.width_stride,
        edge_image_layout.width as usize
    );

    // This iterator zips together RGBA8 and LUMA8 pixels from the source and edge-detected binary image, respectively,
    // allowing us to simply iterate without having to sorry about indexing into raw arrays.
    let zipped_pixel_pair_iterator = target_image_row_contiguous_flat_buffer
        .chunks(target_image_layout.width_stride)
        .zip(edge_image_row_contiguous_flat_buffer)
        .enumerate();

    let mut prepared_segments: Vec<PreparedPixelSortSegment<SortingContext>> =
        Vec::with_capacity(target_image_layout.height as usize);

    let mut current_state: PixelSegmentScannerState<SortingContext> =
        PixelSegmentScannerState::OutsideSortableSegment;

    for (column_index, (target_pixel, edge_pixel)) in zipped_pixel_pair_iterator {
        let target_pixel = retrieve_starting_rgba_pixel_from_flat_samples(target_pixel);

        let belongs_to_segment =
            *edge_pixel == u8::MAX || (column_index == 0 && initial_segment_starts_on_left_edge);

        if belongs_to_segment {
            match current_state {
                PixelSegmentScannerState::OutsideSortableSegment => {
                    let segment_start_index = column_index as u32;
                    let collected_pixels = vec![sorting_context_computation_closure(&target_pixel)];

                    current_state = PixelSegmentScannerState::CollectingSortableSegment {
                        segment_start_index,
                        collected_pixels,
                    };
                }
                PixelSegmentScannerState::CollectingSortableSegment {
                    segment_start_index,
                    mut collected_pixels,
                } => {
                    collected_pixels.push(sorting_context_computation_closure(&target_pixel));

                    current_state = PixelSegmentScannerState::CollectingSortableSegment {
                        segment_start_index,
                        collected_pixels,
                    };
                }
            }
        } else {
            match current_state {
                PixelSegmentScannerState::OutsideSortableSegment => {
                    current_state = PixelSegmentScannerState::OutsideSortableSegment;
                }
                PixelSegmentScannerState::CollectingSortableSegment {
                    segment_start_index,
                    collected_pixels,
                } => {
                    prepared_segments.push(PreparedPixelSortSegment {
                        start_column_index: segment_start_index as usize,
                        pixel_sorting_contexts: collected_pixels,
                    });

                    current_state = PixelSegmentScannerState::OutsideSortableSegment;
                }
            }
        }
    }

    PreparedPixelSortRow {
        sorting_contexts_for_row: prepared_segments,
    }
}


fn prepare_segments_using_detected_edges<SortingContext, SortingContextClosure>(
    target_image: &RgbaImage,
    binary_edge_image: GrayImage,
    sorting_context_computation_closure: SortingContextClosure,
    segment_starts_on_image_edge: bool,
) -> Vec<PreparedPixelSortRow<SortingContext>>
where
    SortingContext: Send + num::Num + Copy + PartialOrd,
    SortingContextClosure: Fn(&Rgba<u8>) -> SortingContext + Send + Sync,
{
    let target_image_layout = target_image.sample_layout();
    let edge_image_layout = binary_edge_image.sample_layout();

    let prepared_rows: Vec<PreparedPixelSortRow<SortingContext>> = binary_edge_image
        .as_flat_samples()
        .as_slice()
        .par_chunks(edge_image_layout.height_stride)
        .zip(
            target_image
                .as_flat_samples()
                .as_slice()
                .par_chunks(target_image_layout.height_stride),
        )
        .map(
            |(binary_edge_image_buffer, target_image_buffer)| {
                prepare_segments_using_detected_edges_for_single_row(
                    target_image_buffer,
                    target_image_layout,
                    binary_edge_image_buffer,
                    edge_image_layout,
                    segment_starts_on_image_edge,
                    &sorting_context_computation_closure,
                )
            },
        )
        .collect();

    prepared_rows
}

fn prepare_axis_aligned_numeric_edge_detected_pixel_sort(
    image: RgbaImage,
    edge_detection_low_threshold: f32,
    edge_detection_high_threshold: f32,
    initial_segment_starts_on_left_image_edge: bool,
    direction: ImageSortingDirection,
    sorting_mode: PreparedSegmentSortingMode,
) -> PreparedPixelSort<f32> {
    match direction {
        ImageSortingDirection::Horizontal(pixel_segment_sort_direction) => {
            let dynamic_image = DynamicImage::ImageRgba8(image);
            let gray_image = dynamic_image.to_luma8();
            let DynamicImage::ImageRgba8(image) = dynamic_image else {
                unreachable!(
                    "this shouldn't be possible, as we just constructed DynamicImage::ImageRgba8 above?!?!"
                );
            };

            let image_edges = imageproc::edges::canny(
                &gray_image,
                edge_detection_low_threshold,
                edge_detection_high_threshold,
            );

            assert!(image_edges.width() == image.width());
            assert!(image_edges.height() == image.height());

            let prepared_row_data = prepare_segments_using_detected_edges(
                &image,
                image_edges,
                |pixel| match sorting_mode {
                    PreparedSegmentSortingMode::Luminance => compute_rgba_relative_luminance(pixel),
                    PreparedSegmentSortingMode::Hue => compute_rgba_hsl_hue(pixel),
                    PreparedSegmentSortingMode::Saturation => compute_rgba_hsl_saturation(pixel),
                },
                initial_segment_starts_on_left_image_edge,
            );

            PreparedPixelSort {
                image: PreparedPixelSortImage::PreparedHorizontal {
                    image,
                    direction: pixel_segment_sort_direction,
                },
                prepared_row_data,
            }
        }
        ImageSortingDirection::Vertical(pixel_segment_sort_direction) => {
            let rotated_image = image::imageops::rotate90(&image);

            let dynamic_image = DynamicImage::ImageRgba8(rotated_image);
            let gray_image = dynamic_image.to_luma8();
            let DynamicImage::ImageRgba8(rotated_image) = dynamic_image else {
                unreachable!(
                    "this shouldn't be possible, as we just constructed DynamicImage::ImageRgba8 above?!?!"
                );
            };

            let image_edges = imageproc::edges::canny(
                &gray_image,
                edge_detection_low_threshold,
                edge_detection_high_threshold,
            );

            assert!(image_edges.width() == rotated_image.width());
            assert!(image_edges.height() == rotated_image.height());

            let prepared_row_data = prepare_segments_using_detected_edges(
                &rotated_image,
                image_edges,
                |pixel| match sorting_mode {
                    PreparedSegmentSortingMode::Luminance => compute_rgba_relative_luminance(pixel),
                    PreparedSegmentSortingMode::Hue => compute_rgba_hsl_hue(pixel),
                    PreparedSegmentSortingMode::Saturation => compute_rgba_hsl_saturation(pixel),
                },
                initial_segment_starts_on_left_image_edge,
            );

            PreparedPixelSort {
                image: PreparedPixelSortImage::PreparedHorizontal {
                    image: rotated_image,
                    direction: pixel_segment_sort_direction,
                },
                prepared_row_data,
            }
        }
    }
}


pub fn prepare_pixel_sort(
    image: RgbaImage,
    selection_mode: PreparedSegmentSelectionMode,
    sorting_mode: PreparedSegmentSortingMode,
    direction: ImageSortingDirection,
) -> PreparedPixelSort<f32> {
    match selection_mode {
        PreparedSegmentSelectionMode::LuminanceRange { low, high } => {
            let target_luminance_range = low..=high;

            prepare_axis_aligned_numeric_pixel_sort(
                image,
                direction,
                |pixel: &Rgba<u8>| -> f32 { compute_rgba_relative_luminance(pixel) },
                |pixel: &PixelWithContext<f32>| -> bool {
                    target_luminance_range.contains(&pixel.context)
                },
                |pixel| match sorting_mode {
                    PreparedSegmentSortingMode::Luminance => pixel.context,
                    PreparedSegmentSortingMode::Hue => compute_rgba_hsl_hue(&pixel.pixel),
                    PreparedSegmentSortingMode::Saturation => {
                        compute_rgba_hsl_saturation(&pixel.pixel)
                    }
                },
            )
        }
        PreparedSegmentSelectionMode::HueRange { low, high } => {
            let target_hue_range = low..=high;

            prepare_axis_aligned_numeric_pixel_sort(
                image,
                direction,
                |pixel: &Rgba<u8>| -> f32 { compute_rgba_hsl_hue(pixel) },
                |pixel: &PixelWithContext<f32>| -> bool {
                    target_hue_range.contains(&pixel.context)
                },
                |pixel| match sorting_mode {
                    PreparedSegmentSortingMode::Luminance => {
                        compute_rgba_relative_luminance(&pixel.pixel)
                    }
                    PreparedSegmentSortingMode::Hue => pixel.context,
                    PreparedSegmentSortingMode::Saturation => {
                        compute_rgba_hsl_saturation(&pixel.pixel)
                    }
                },
            )
        }
        PreparedSegmentSelectionMode::SaturationRange { low, high } => {
            let target_saturation_range = low..=high;

            prepare_axis_aligned_numeric_pixel_sort(
                image,
                direction,
                |pixel: &Rgba<u8>| -> f32 { compute_rgba_hsl_saturation(pixel) },
                |pixel: &PixelWithContext<f32>| -> bool {
                    target_saturation_range.contains(&pixel.context)
                },
                |pixel| match sorting_mode {
                    PreparedSegmentSortingMode::Luminance => {
                        compute_rgba_relative_luminance(&pixel.pixel)
                    }
                    PreparedSegmentSortingMode::Hue => compute_rgba_hsl_hue(&pixel.pixel),
                    PreparedSegmentSortingMode::Saturation => pixel.context,
                },
            )
        }
        PreparedSegmentSelectionMode::CannyEdges {
            low,
            high,
            segment_starts_on_image_edge: initial_segment_starts_on_image_edge,
        } => prepare_axis_aligned_numeric_edge_detected_pixel_sort(
            image,
            low,
            high,
            initial_segment_starts_on_image_edge,
            direction,
            sorting_mode,
        ),
    }
}


pub fn modify_prepared_pixel_sort_segments_with<SortingContext, SegmentsClosure>(
    mut prepared_pixel_sort: PreparedPixelSort<SortingContext>,
    mut segment_modification_closure: SegmentsClosure,
) -> PreparedPixelSort<SortingContext>
where
    SortingContext: Send + num::Num + Copy + PartialOrd,
    SegmentsClosure: FnMut(&mut Vec<PreparedPixelSortRow<SortingContext>>),
{
    segment_modification_closure(&mut prepared_pixel_sort.prepared_row_data);
    prepared_pixel_sort
}


// TODO write random splitter of segments, then integrate it into the GUI

pub enum SegmentRandomizationMode {
    Uniform {
        low_inclusive: usize,
        high_inclusive: usize,
    },

    Normal {
        // Also known as mu.
        mean: f32,

        // Also known as sigma.
        standard_deviation: f32,
    },
}


pub fn randomize_prepared_segments<SortingContext>(
    prepared_pixel_sort: PreparedPixelSort<SortingContext>,
    mode: SegmentRandomizationMode,
) -> PreparedPixelSort<SortingContext>
where
    SortingContext: Send + num::Num + Copy + PartialOrd,
{
    let mut thread_rng = rand::rng();

    let image = prepared_pixel_sort.image;
    let mut randomized_prepared_rows =
        Vec::with_capacity(prepared_pixel_sort.prepared_row_data.len());

    for row in prepared_pixel_sort.prepared_row_data {
        let mut randomized_row_data = Vec::with_capacity(row.sorting_contexts_for_row.len());

        for original_segment in row.sorting_contexts_for_row {
            let number_of_pixels_in_segment = original_segment.pixel_sorting_contexts.len();
            let mut current_pixel_offset = 0usize;

            match mode {
                SegmentRandomizationMode::Uniform {
                    low_inclusive,
                    high_inclusive,
                } => {
                    assert!(low_inclusive <= high_inclusive);
                    let distribution = Uniform::new_inclusive(low_inclusive, high_inclusive)
                                        .expect("initialization error is impossible, since low is never higher than high");

                    while current_pixel_offset < number_of_pixels_in_segment {
                        let pixels_left = number_of_pixels_in_segment - current_pixel_offset;
                        let target_segment_length =
                            distribution.sample(&mut thread_rng).min(pixels_left);

                        let mut randomized_partial_segment: Vec<SortingContext> =
                            Vec::with_capacity(target_segment_length);

                        for pixel_offset_index in 0..target_segment_length {
                            let segment_context = original_segment.pixel_sorting_contexts
                                [current_pixel_offset + pixel_offset_index];

                            randomized_partial_segment.push(segment_context);
                        }

                        randomized_row_data.push(PreparedPixelSortSegment {
                            start_column_index: original_segment.start_column_index
                                + current_pixel_offset,
                            pixel_sorting_contexts: randomized_partial_segment,
                        });

                        current_pixel_offset += target_segment_length;
                    }
                }
                SegmentRandomizationMode::Normal {
                    mean,
                    standard_deviation,
                } => {
                    assert!(standard_deviation.is_finite());
                    let distribution = Normal::new(mean, standard_deviation)
                        .expect("unusable mean and standard deviation");

                    while current_pixel_offset < number_of_pixels_in_segment {
                        let pixels_left = number_of_pixels_in_segment - current_pixel_offset;
                        let target_segment_length = (distribution.sample(&mut thread_rng).round()
                            as usize)
                            .min(pixels_left);

                        let mut randomized_partial_segment: Vec<SortingContext> =
                            Vec::with_capacity(target_segment_length);

                        for pixel_offset_index in 0..target_segment_length {
                            let segment_context = original_segment.pixel_sorting_contexts
                                [current_pixel_offset + pixel_offset_index];

                            randomized_partial_segment.push(segment_context);
                        }

                        randomized_row_data.push(PreparedPixelSortSegment {
                            start_column_index: original_segment.start_column_index
                                + current_pixel_offset,
                            pixel_sorting_contexts: randomized_partial_segment,
                        });

                        current_pixel_offset += target_segment_length;
                    }
                }
            }
        }

        randomized_prepared_rows.push(PreparedPixelSortRow {
            sorting_contexts_for_row: randomized_row_data,
        });
    }

    PreparedPixelSort {
        image,
        prepared_row_data: randomized_prepared_rows,
    }
}



fn execute_prepared_pixel_sort_on_image_row<SortingContext>(
    image_row_contiguous_flat_buffer: &mut [u8],
    image_layout: SampleLayout,
    sorting_direction: PixelSegmentSortDirection,
    prepared_row: PreparedPixelSortRow<SortingContext>,
) where
    SortingContext: Send + num::Num + Copy + PartialOrd,
{
    let image_channel_stride = image_layout.channel_stride;
    let image_number_of_channels = image_layout.channels as usize;

    for segment in prepared_row.sorting_contexts_for_row {
        let start_column_index = segment.start_column_index;

        let (_, realigned_row_slice) = image_row_contiguous_flat_buffer
            .split_at_mut(start_column_index * image_channel_stride * image_number_of_channels);

        let (clipped_segment_slice, _) = realigned_row_slice.split_at_mut(
            segment.pixel_sorting_contexts.len() * image_channel_stride * image_number_of_channels,
        );

        let contextualized_pixels: Vec<PixelWithContext<SortingContext>> = clipped_segment_slice
            .par_chunks(image_layout.width_stride)
            .map(|pixel_data| Rgba([pixel_data[0], pixel_data[1], pixel_data[2], pixel_data[3]]))
            .zip(segment.pixel_sorting_contexts)
            .map(|(pixel, sorting_context)| PixelWithContext::new(pixel, sorting_context))
            .collect();

        sort_with_numeric_context_and_reapply_pixel_segment(
            contextualized_pixels,
            sorting_direction,
            clipped_segment_slice,
            image_layout,
        );
    }
}


#[allow(clippy::let_and_return)]
pub fn execute_axis_aligned_prepared_pixel_sort<SortingContext>(
    prepared_pixel_sort: PreparedPixelSort<SortingContext>,
) -> RgbaImage
where
    SortingContext: Send + num::Num + Copy + PartialOrd,
{
    match prepared_pixel_sort.image {
        PreparedPixelSortImage::PreparedHorizontal {
            mut image,
            direction,
        } => {
            assert!(prepared_pixel_sort.prepared_row_data.len() == image.height() as usize);

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
            flat_samples
                .as_mut_slice()
                .par_chunks_mut(image_layout.height_stride)
                .zip(prepared_pixel_sort.prepared_row_data)
                .for_each(|(row_buffer, prepared_segments)| {
                    execute_prepared_pixel_sort_on_image_row(
                        row_buffer,
                        image_layout,
                        direction,
                        prepared_segments,
                    );
                });

            image
        }
        PreparedPixelSortImage::PreparedVertical {
            mut rotated_image,
            direction,
        } => {
            assert_eq!(
                prepared_pixel_sort.prepared_row_data.len(),
                rotated_image.height() as usize
            );

            // For performance reasons, we'll operate directly on the underlying RGBA8 image buffer.
            let mut flat_samples = rotated_image.as_flat_samples_mut();

            // This is known to us, since we are expecting RGBA8.
            // Still, we'll use the values from the `layout` struct directly from here on.
            assert!(!flat_samples.has_aliased_samples());
            assert!(flat_samples.layout.channel_stride == 1);
            assert!(flat_samples.layout.channels == 4);

            let image_layout = flat_samples.layout;

            // The pixel sorting is performed here in parallel for each row of the image
            // using `rayon`'s parallel iterators.
            flat_samples
                .as_mut_slice()
                .par_chunks_mut(image_layout.height_stride)
                .zip(prepared_pixel_sort.prepared_row_data)
                .for_each(|(row_buffer, prepared_segments)| {
                    execute_prepared_pixel_sort_on_image_row(
                        row_buffer,
                        image_layout,
                        direction,
                        prepared_segments,
                    );
                });

            let inverse_rotated_image = image::imageops::rotate270(&rotated_image);

            inverse_rotated_image
        }
    }
}
