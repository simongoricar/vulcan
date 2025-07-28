use image::{Rgba, flat::SampleLayout};

pub mod immediate;
pub mod prepared;
mod properties;
mod sorting;


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



/// An internal struct that carries contextual information (e.g. relative luminance)
/// alongside the actual [`Rgba`]`<`[`u8`]`>` pixel value.
#[derive(Debug, Clone, PartialEq)]
pub struct PixelWithContext<C> {
    pub pixel: Rgba<u8>,
    pub context: C,
}

impl<C> PixelWithContext<C> {
    #[inline(always)]
    pub fn new(pixel: Rgba<u8>, context: C) -> Self {
        Self { pixel, context }
    }
}

impl<C> AsRef<Rgba<u8>> for PixelWithContext<C> {
    fn as_ref(&self) -> &Rgba<u8> {
        &self.pixel
    }
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



// /// Adapted from [this Stack Overflow post](https://stackoverflow.com/questions/23090019/fastest-formula-to-get-hue-from-rgb).
// fn get_rgba_pixel_hue(pixel: &Rgba<u8>) -> f64 {
//     let [r, g, b, _] = pixel.0;
//     let r = r as f64;
//     let g = g as f64;
//     let b = b as f64;

//     let minimum = r.min(g).min(b);
//     let maximum = r.max(g).max(b);

//     let raw_hue_value = if r == maximum {
//         (g - b) / (maximum / minimum)
//     } else if g == maximum {
//         2.0 + (b - r) / (maximum - minimum)
//     } else {
//         // (blue is maximum)
//         4.0 + (r - g) / (maximum - minimum)
//     };

//     if raw_hue_value < 0.0 {
//         raw_hue_value * 60.0 + 360.0
//     } else {
//         raw_hue_value * 60.0
//     }
// }

// pub struct PixelSortOutput {
//     output_image: DynamicImage,
//     detected_edges: ImageBuffer<Luma<u8>, Vec<u8>>,
// }

// fn perform_horizontal_pixel_sort(
//     mut input_image: DynamicImage,
// ) -> miette::Result<PixelSortOutput> {
//     let height = input_image.height();
//     let width = input_image.width();

//     info!("Converting image to luma.");
//     let gray_image = input_image.to_luma8();
//     // info!("Applying gaussian blur.");
//     // let blurred_image = imageproc::filter::gaussian_blur_f32(&gray_image, 1.0);

//     let mut thresholded_image =
//         imageproc::contrast::threshold(&gray_image, 180, ThresholdType::ToZero);

//     // Invert threshold
//     // thresholded_image
//     //     .pixels_mut()
//     //     .for_each(|pixel| pixel.invert());

//     // info!("Detecting image edges.");
//     // let edge_detected_image = imageproc::edges::canny(&blurred_image, 4.0, 80.0);

//     // Construct inversion list - segements that need to be sorted
//     let mut horizontal_segments_per_line_index =
//         Vec::with_capacity(height as usize);
//     let mut vertical_segments_per_column_index =
//         Vec::with_capacity(height as usize);

//     // Compute horizontal segments.
//     for y_index in 0..height {
//         let mut segments: InversionMap<u32, bool> = InversionMap::new();
//         segments.insert(0..width, false).into_diagnostic()?;

//         let mut segment_start_index: Option<u32> = None;

//         for x_index in 0..width {
//             let pixel = thresholded_image.get_pixel(x_index, y_index);
//             let is_active = pixel.0[0].eq(&255);

//             if !is_active {
//                 if let Some(segment_start_index) = segment_start_index {
//                     // Active segment has ended.
//                     segments
//                         .insert(segment_start_index..x_index, true)
//                         .into_diagnostic()?;
//                 }
//             } else if is_active && segment_start_index.is_none() {
//                 segment_start_index = Some(x_index);
//             }
//         }

//         horizontal_segments_per_line_index.push(segments);
//     }

//     // Compute vertical segments.
//     for x_index in 0..width {
//         let mut segments: InversionMap<u32, bool> = InversionMap::new();
//         segments.insert(0..width, false).into_diagnostic()?;

//         let mut segment_start_index: Option<u32> = None;

//         for y_index in 0..height {
//             let pixel = thresholded_image.get_pixel(x_index, y_index);
//             let is_active = pixel.0[0].eq(&255);

//             if !is_active {
//                 if let Some(segment_start_index) = segment_start_index {
//                     // Active segment has ended.
//                     segments
//                         .insert(segment_start_index..y_index, true)
//                         .into_diagnostic()?;
//                 }
//             } else if is_active && segment_start_index.is_none() {
//                 segment_start_index = Some(y_index);
//             }
//         }

//         vertical_segments_per_column_index.push(segments);
//     }

//     // Perform vertical pixelsort.
//     /*
//        for x_index in 0..width {
//            let segments = vertical_segments_per_column_index
//                .get(x_index as usize)
//                .expect("bug: x index should have been present");

//            for segment in segments.iter() {
//                if !segment.value {
//                    continue;
//                }

//                let mut pixels = segment
//                    .range()
//                    .clone()
//                    .map(|vertical_index| {
//                        input_image.get_pixel(x_index, vertical_index)
//                    })
//                    .collect::<Vec<_>>();

//                pixels.sort_unstable_by(|first, second| {
//                    let first_hue = get_rgba_pixel_hue(first);
//                    let second_hue = get_rgba_pixel_hue(second);

//                    first_hue.total_cmp(&second_hue)
//                });

//                // Reapply the pixels onto the image
//                for (y_index, pixel_value) in pixels.into_iter().enumerate() {
//                    input_image.put_pixel(
//                        x_index,
//                        *segment.start_inclusive() + y_index as u32,
//                        pixel_value,
//                    );
//                }
//            }
//        }
//     */
//     // Perform the horizontal pixel sort.
//     // TODO Use the segments.
//     for y_index in 0..height {
//         let segments = horizontal_segments_per_line_index
//             .get(y_index as usize)
//             .expect("bug: y index should have been present");

//         for segment in segments.iter() {
//             if !segment.value {
//                 continue;
//             }

//             let mut pixels = segment
//                 .range()
//                 .clone()
//                 .map(|horizontal_index| {
//                     input_image.get_pixel(horizontal_index, y_index)
//                 })
//                 .collect::<Vec<_>>();

//             pixels.sort_unstable_by(|first, second| {
//                 let first_hue = get_rgba_pixel_hue(first);
//                 let second_hue = get_rgba_pixel_hue(second);

//                 first_hue.total_cmp(&second_hue)
//             });

//             // Reapply the pixels onto the image
//             for (x_index, pixel_value) in pixels.into_iter().enumerate() {
//                 input_image.put_pixel(
//                     *segment.start_inclusive() + x_index as u32,
//                     y_index,
//                     pixel_value,
//                 );
//             }
//         }
//     }

//     Ok(PixelSortOutput {
//         output_image: input_image,
//         detected_edges: thresholded_image,
//     })
// }

// pub fn cmd_generate(args: GenerateArgs) -> miette::Result<()> {
//     info!("Reading input image.");
//     let input_image = ImageReader::open(args.input_image_path)
//         .into_diagnostic()
//         .wrap_err_with(|| miette!("Failed to open input image."))?
//         .decode()
//         .into_diagnostic()
//         .wrap_err_with(|| miette!("Failed to decode input image."))?;

//     // info!("Converting image to luma.");
//     // let gray_image = input_image.into_luma8();
//     //
//     // info!("Applying gaussian blur.");
//     // let blurred_image = imageproc::filter::gaussian_blur_f32(&gray_image, 1.0);
//     //
//     // info!("Detecting image edges.");
//     // let output_image = imageproc::edges::canny(&blurred_image, 4.0, 80.0);

//     let pixel_sort_outputs = perform_horizontal_pixel_sort(input_image)
//         .wrap_err_with(|| miette!("Failed to perform horizontal pixel sort."))?;

//     info!("Saving outputs.");

//     save_image_as_png(
//         pixel_sort_outputs.detected_edges,
//         args.output_image_path
//             .with_suffix_to_stem("_edges")
//             .ok_or_else(|| miette!("Failed to construct output path."))?,
//         true,
//     )
//     .wrap_err_with(|| miette!("Failed to save edge image."))?;

//     save_image_as_png(
//         pixel_sort_outputs.output_image,
//         args.output_image_path,
//         true,
//     )
//     .wrap_err_with(|| miette!("Failed to save output image."))?;

//     Ok(())
// }
