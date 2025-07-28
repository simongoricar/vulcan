use std::ops::Rem;

use image::Rgba;
use num::Zero;



/// Converts a gamma-encoded `u8` sRGB value to a linear `u8` sRGB value.
///
/// See <https://en.wikipedia.org/wiki/Relative_luminance> for more information.
///
/// TODO This can be improved: ^2.2 does not fully match the gamma->linear conversion,
///      see the transfer function here: <https://en.wikipedia.org/wiki/SRGB>
///      and here <https://stackoverflow.com/questions/596216/formula-to-determine-perceived-brightness-of-rgb-color>.
#[inline(always)]
pub fn convert_gamma_encoded_srgb_to_linear(value: u8) -> u8 {
    let input_value_as_f32 = value as f32 / u8::MAX as f32;
    let output_value_as_f32 = input_value_as_f32.powf(2.2);

    (output_value_as_f32 * u8::MAX as f32) as u8
}


/// Computes the relative luminance[^relative-luminance] of an RGBA pixel,
/// as an `f32` in the range `0.0..=1.0`.
///
///
/// [^relative-luminance]: See <https://www.w3.org/WAI/GL/wiki/Relative_luminance> for more information.
pub fn compute_rgba_relative_luminance(pixel: &Rgba<u8>) -> f32 {
    let linear_r = convert_gamma_encoded_srgb_to_linear(pixel.0[0]);
    let linear_g = convert_gamma_encoded_srgb_to_linear(pixel.0[1]);
    let linear_b = convert_gamma_encoded_srgb_to_linear(pixel.0[2]);

    let relative_luminance_up_to_u8_range = 0.2126f32 * (linear_r as f32)
        + 0.7152f32 * (linear_g as f32)
        + 0.0722f32 * (linear_b as f32);

    relative_luminance_up_to_u8_range / (u8::MAX as f32)
}


#[allow(clippy::let_and_return)]
pub fn compute_rgba_hsl_hue(pixel: &Rgba<u8>) -> f32 {
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


#[allow(clippy::let_and_return)]
pub fn compute_rgba_hsl_lightness(pixel: &Rgba<u8>) -> f32 {
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


#[allow(clippy::let_and_return)]
pub fn compute_rgba_hsl_saturation(pixel: &Rgba<u8>) -> f32 {
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
