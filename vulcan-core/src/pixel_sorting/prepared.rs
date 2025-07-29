use image::{Rgba, RgbaImage, flat::SampleLayout};
use rayon::prelude::{ParallelIterator, ParallelSlice};

use crate::pixel_sorting::{
    ImageSortingDirection,
    PixelSegmentScannerState,
    PixelSegmentSortDirection,
    PixelWithContext,
    retrieve_rgba_pixel_from_flat_samples,
};

pub enum TwoPassSegmentSelectionMode {
    /// This mode creates pixel sorting segments that consist *only* of
    /// continuous pixels whose relative luminance[^relative-luminance]
    /// is between `low` and `high` (both inclusive).
    ///
    ///
    /// [^relative-luminance]: See [this Wikipedia article](https://en.wikipedia.org/wiki/Relative_luminance) for more information.
    // LuminanceRange {
    //     /// The inclusive low end of the relative luminance range (`0.0..=1.0`).
    //     low: f32,

    //     /// The inclusive high end of the relative luminance range (`0.0..=1.0`).
    //     high: f32,
    // },

    // HueRange {
    //     /// The inclusive low end of the hue range (`0.0..360.0`).
    //     low: f32,

    //     /// The inclusive high end of the hue range (`0.0..360.0`).
    //     high: f32,
    // },

    // SaturationRange {
    //     /// The inclusive low end of the saturation range (`0.0..=1.0`).
    //     low: f32,

    //     /// The inclusive high end of the saturation range (`0.0..=1.0`).
    //     high: f32,
    // },
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

pub enum PreparedPixelSortImage {
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

pub struct PreparedPixelSort<SortingContext>
where
    SortingContext: Send,
{
    image: PreparedPixelSortImage,

    /// These are the custom sorting contexts, presented in row-major order.
    sorting_contexts: Vec<Vec<PreparedPixelSortSegment<SortingContext>>>,
}

struct PreparedPixelSortSegment<SortingContext>
where
    SortingContext: Send,
{
    start_column_index: usize,
    pixel_sorting_contexts: Vec<SortingContext>,
}

impl<SortingContext> PreparedPixelSortSegment<SortingContext>
where
    SortingContext: Send,
{
    fn segment_length(&self) -> usize {
        self.pixel_sorting_contexts.len()
    }
}

struct PreparedPixelSortRow<SortingContext>
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

fn prepare_axis_aligned_generic_pixel_sort<
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
    SortingContext: Send,
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
            let mut sorting_contexts: Vec<Vec<PreparedPixelSortSegment<SortingContext>>> =
                Vec::with_capacity(prepared_rows.len());

            for row_data in prepared_rows {
                sorting_contexts.push(row_data.sorting_contexts_for_row);
            }

            PreparedPixelSort {
                image: PreparedPixelSortImage::PreparedHorizontal {
                    image,
                    direction: pixel_segment_sort_direction,
                },
                sorting_contexts,
            }
        }
        ImageSortingDirection::Vertical(pixel_segment_sort_direction) => {
            let rotated_image = image::imageops::rotate90(&image);

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
            let mut sorting_contexts: Vec<Vec<PreparedPixelSortSegment<SortingContext>>> =
                Vec::with_capacity(prepared_rows.len());

            for row_data in prepared_rows {
                sorting_contexts.push(row_data.sorting_contexts_for_row);
            }

            PreparedPixelSort {
                image: PreparedPixelSortImage::PreparedVertical {
                    rotated_image,
                    direction: pixel_segment_sort_direction,
                },
                sorting_contexts,
            }
        }
    }
}

fn execute_prepared_pixel_sort_on_image_row<SortingContext>(
    image_row_contiguous_buffer: &mut [u8],
    image_layout: SampleLayout,
    prepared_row: PreparedPixelSortRow<SortingContext>,
) where
    SortingContext: Send,
{
    for segment in prepared_row.sorting_contexts_for_row {}

    todo!();
}

fn execute_axis_aligned_prepared_pixel_sort<SortingContext, SortingClosure>(
    prepared_pixel_sort: PreparedPixelSort<SortingContext>,
    mut segment_sorting_closure: SortingClosure,
) where
    SortingContext: Send,
    SortingClosure: FnMut(&mut [PixelWithContext<SortingContext>]),
{
    todo!();
}
