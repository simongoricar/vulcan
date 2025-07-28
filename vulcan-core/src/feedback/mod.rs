use image::{Rgba, RgbaImage};
use rayon::prelude::ParallelIterator;

use crate::pixel_sorting::properties::{
    compute_rgba_hsl_hue,
    compute_rgba_hsl_saturation,
    compute_rgba_relative_luminance,
};

pub enum FeedbackSegmentSelectionMode {
    LuminanceRange { low: f32, high: f32 },
    HueRange { low: f32, high: f32 },
    SaturationRange { low: f32, high: f32 },
}

fn mask_out_non_targeted_pixels_using<SegmentMembershipClosure>(
    image: &mut RgbaImage,
    non_targeted_pixels_color: Rgba<u8>,
    segment_membership_closure: SegmentMembershipClosure,
) where
    SegmentMembershipClosure: Fn(&Rgba<u8>) -> bool + Send + Sync,
{
    image.par_pixels_mut().for_each(|pixel| {
        let would_be_sorted = segment_membership_closure(pixel);

        if !would_be_sorted {
            *pixel = non_targeted_pixels_color;
        }
    });
}



pub const PIXEL_BLACK: Rgba<u8> = Rgba([0, 0, 0, u8::MAX]);


pub fn mask_out_non_targeted_pixels(
    image: &mut RgbaImage,
    mode: FeedbackSegmentSelectionMode,
    non_targeted_pixels_color: Rgba<u8>,
) {
    match mode {
        FeedbackSegmentSelectionMode::LuminanceRange { low, high } => {
            let target_luminance_range = low..=high;

            mask_out_non_targeted_pixels_using(
                image,
                non_targeted_pixels_color,
                |pixel| {
                    let relative_luminance =
                        compute_rgba_relative_luminance(pixel);

                    target_luminance_range.contains(&relative_luminance)
                },
            )
        }
        FeedbackSegmentSelectionMode::HueRange { low, high } => {
            let target_hue_range = low..=high;

            mask_out_non_targeted_pixels_using(
                image,
                non_targeted_pixels_color,
                |pixel| {
                    let hue = compute_rgba_hsl_hue(pixel);

                    target_hue_range.contains(&hue)
                },
            )
        }
        FeedbackSegmentSelectionMode::SaturationRange { low, high } => {
            let target_saturation_range = low..=high;

            mask_out_non_targeted_pixels_using(
                image,
                non_targeted_pixels_color,
                |pixel| {
                    let saturation = compute_rgba_hsl_saturation(pixel);

                    target_saturation_range.contains(&saturation)
                },
            )
        }
    }
}
