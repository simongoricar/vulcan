use std::cmp::Ordering;

use image::flat::SampleLayout;

use crate::pixel_sorting::{
    PixelSegmentSortDirection,
    PixelWithContext,
    copy_pixel_segment_onto_image,
};

/// Sorts the given contextualized `pixels` using the sorting closure,
/// then copies the sorted pixels onto the target image, provided as a flat RGBA8 buffer
/// (`target_image_contiguous_flat_buffer`).
///
/// # Panics
/// The length of `target_image_contiguous_flat_buffer` must be precisely large
/// enough to fit all the `source_pixels`; the function will otherwise panic.
pub fn sort_with_closure_and_reapply_pixel_segment<C, S>(
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
#[deprecated = "use sort_with_closure_and_reapply_pixel_segment instead"]
pub fn sort_with_numeric_context_and_reapply_pixel_segment<C>(
    mut pixels: Vec<PixelWithContext<C>>,
    sort_direction: PixelSegmentSortDirection,
    target_image_contiguous_flat_buffer: &mut [u8],
    target_image_layout: SampleLayout,
) where
    C: num::Num + Copy + PartialOrd,
{
    assert!(
        pixels.len() * target_image_layout.channel_stride * target_image_layout.channels as usize
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
