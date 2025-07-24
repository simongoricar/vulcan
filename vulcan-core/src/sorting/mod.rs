mod v2;
pub use v2::*;

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
