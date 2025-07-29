use std::ops::Rem;

use image::Rgba;
use num::Zero;


/// Converts a gamma-encoded `u8` (`0..=255`) sRGB value to a linear `f32` (`0.0..=1.0`) sRGB value.
///
/// See <https://en.wikipedia.org/wiki/Relative_luminance> for more information.
///
/// TODO This can be improved: ^2.2 does not fully match the gamma->linear conversion,
///      see the transfer function here: <https://en.wikipedia.org/wiki/SRGB>
///      and here <https://stackoverflow.com/questions/596216/formula-to-determine-perceived-brightness-of-rgb-color>.
#[inline(always)]
fn convert_gamma_encoded_srgb_u8_to_linear_f32(value: u8) -> f32 {
    let input_value_as_f32 = value as f32 / u8::MAX as f32;
    input_value_as_f32.powf(2.2)
}


/// Computes the relative luminance[^relative-luminance] of an RGBA pixel,
/// as an `f32` in the range `0.0..=1.0`.
///
///
/// [^relative-luminance]: See <https://www.w3.org/WAI/GL/wiki/Relative_luminance> for more information.
#[allow(clippy::let_and_return)]
pub fn compute_rgba_relative_luminance(pixel: &Rgba<u8>) -> f32 {
    let linear_r = convert_gamma_encoded_srgb_u8_to_linear_f32(pixel.0[0]);
    let linear_g = convert_gamma_encoded_srgb_u8_to_linear_f32(pixel.0[1]);
    let linear_b = convert_gamma_encoded_srgb_u8_to_linear_f32(pixel.0[2]);

    let relative_luminance_up_to_u8_range =
        0.2126f32 * linear_r + 0.7152f32 * linear_g + 0.0722f32 * linear_b;

    relative_luminance_up_to_u8_range
}


#[allow(clippy::let_and_return)]
pub fn compute_rgba_hsl_hue(pixel: &Rgba<u8>) -> f32 {
    let linear_r = convert_gamma_encoded_srgb_u8_to_linear_f32(pixel.0[0]);
    let linear_g = convert_gamma_encoded_srgb_u8_to_linear_f32(pixel.0[1]);
    let linear_b = convert_gamma_encoded_srgb_u8_to_linear_f32(pixel.0[2]);

    let max_value = linear_r.max(linear_g).max(linear_b);
    let min_value = linear_r.min(linear_g).min(linear_b);

    let chroma = max_value - min_value;

    let hue_prime = if chroma.is_zero() {
        0f32
    } else if max_value == linear_r {
        ((linear_g - linear_b) / chroma).rem(6f32)
    } else if max_value == linear_g {
        ((linear_b - linear_r) / chroma) + 2f32
    } else if max_value == linear_b {
        ((linear_r - linear_g) / chroma) + 4f32
    } else {
        unreachable!();
    };

    let hue = hue_prime * 60f32;

    hue
}


#[allow(clippy::let_and_return)]
pub fn compute_rgba_hsl_lightness(pixel: &Rgba<u8>) -> f32 {
    let linear_r = convert_gamma_encoded_srgb_u8_to_linear_f32(pixel.0[0]);
    let linear_g = convert_gamma_encoded_srgb_u8_to_linear_f32(pixel.0[1]);
    let linear_b = convert_gamma_encoded_srgb_u8_to_linear_f32(pixel.0[2]);

    let max_value = linear_r.max(linear_g).max(linear_b);
    let min_value = linear_r.min(linear_g).min(linear_b);

    let lightness = (max_value + min_value) / 2f32;

    lightness
}


#[allow(clippy::let_and_return)]
pub fn compute_rgba_hsl_saturation(pixel: &Rgba<u8>) -> f32 {
    let linear_r = convert_gamma_encoded_srgb_u8_to_linear_f32(pixel.0[0]);
    let linear_g = convert_gamma_encoded_srgb_u8_to_linear_f32(pixel.0[1]);
    let linear_b = convert_gamma_encoded_srgb_u8_to_linear_f32(pixel.0[2]);

    let max_value = linear_r.max(linear_g).max(linear_b);
    let min_value = linear_r.min(linear_g).min(linear_b);

    let lightness = (max_value + min_value) / 2f32;

    // See <https://en.wikipedia.org/wiki/HSL_and_HSV#From_RGB>
    let saturation = if lightness == 0f32 || lightness == 1f32 {
        0f32
    } else {
        (max_value - lightness) / lightness.min(1f32 - lightness)
    };

    saturation
}
