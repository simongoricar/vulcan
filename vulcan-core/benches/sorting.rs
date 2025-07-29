use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use image::{Rgba, RgbaImage};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use vulcan_core::pixel_sorting::{
    ImageSortingDirection,
    PixelSegmentSortDirection,
    immediate::{ImmediateSegmentSelectionMode, PixelSortOptions, perform_pixel_sort},
};

const SAMPLE_IMAGE_WIDTH: u32 = 512;
const SAMPLE_IMAGE_HEIGHT: u32 = 512;

const SAMPLE_IMAGE_SEEDS: [u64; 8] = [
    205536008065667967,
    4273966252279457180,
    8051738156528302057,
    12807982389205187129,
    17325511456879497815,
    10073918629329680582,
    14696477226502723511,
    14417117096341464374,
];

fn generate_input_image(seed: u64) -> RgbaImage {
    let mut generator = ChaCha8Rng::seed_from_u64(seed);

    let mut image = RgbaImage::new(SAMPLE_IMAGE_WIDTH, SAMPLE_IMAGE_HEIGHT);

    for column_index in 0..SAMPLE_IMAGE_WIDTH {
        for row_index in 0..SAMPLE_IMAGE_HEIGHT {
            let red = generator.random::<u8>();
            let green = generator.random::<u8>();
            let blue = generator.random::<u8>();

            image.put_pixel(
                row_index,
                column_index,
                Rgba([red, green, blue, u8::MAX]),
            );
        }
    }

    image
}

#[derive(Debug, Clone)]
struct TestImages([RgbaImage; 8]);

impl TestImages {
    pub fn generate() -> Self {
        Self([
            generate_input_image(SAMPLE_IMAGE_SEEDS[0]),
            generate_input_image(SAMPLE_IMAGE_SEEDS[1]),
            generate_input_image(SAMPLE_IMAGE_SEEDS[2]),
            generate_input_image(SAMPLE_IMAGE_SEEDS[3]),
            generate_input_image(SAMPLE_IMAGE_SEEDS[4]),
            generate_input_image(SAMPLE_IMAGE_SEEDS[5]),
            generate_input_image(SAMPLE_IMAGE_SEEDS[6]),
            generate_input_image(SAMPLE_IMAGE_SEEDS[7]),
        ])
    }

    pub fn into_images(self) -> [RgbaImage; 8] {
        self.0
    }
}

fn luminance_range_sort_benchmark(c: &mut Criterion) {
    const LUMINANCE_THRESHOLD_LOW: f32 = 0.15;
    const LUMINANCE_THRESHOLD_HIGH: f32 = 0.85;

    let test_images = TestImages::generate();

    c.bench_with_input(
        BenchmarkId::new(
            "luminance range sorting, horizontal ascending (512x512, 2/3 luma range)",
            1,
        ),
        &test_images,
        |bencher, input| {
            bencher.iter_batched(
                || input.to_owned(),
                |input| {
                    for image in input.into_images() {
                        perform_pixel_sort(
                            image,
                            ImmediateSegmentSelectionMode::LuminanceRange {
                                low: LUMINANCE_THRESHOLD_LOW,
                                high: LUMINANCE_THRESHOLD_HIGH,
                            },
                            PixelSortOptions {
                                direction: ImageSortingDirection::Horizontal(
                                    PixelSegmentSortDirection::Ascending,
                                ),
                            },
                        );
                    }
                },
                BatchSize::SmallInput,
            );
        },
    );

    c.bench_with_input(
        BenchmarkId::new(
            "luminance range sorting, vertical ascending (512x512, 2/3 luma range)",
            1,
        ),
        &test_images,
        |bencher, input| {
            bencher.iter_batched(
                || input.to_owned(),
                |input| {
                    for image in input.into_images() {
                        perform_pixel_sort(
                            image,
                            ImmediateSegmentSelectionMode::LuminanceRange {
                                low: LUMINANCE_THRESHOLD_LOW,
                                high: LUMINANCE_THRESHOLD_HIGH,
                            },
                            PixelSortOptions {
                                direction: ImageSortingDirection::Vertical(
                                    PixelSegmentSortDirection::Ascending,
                                ),
                            },
                        );
                    }
                },
                BatchSize::SmallInput,
            );
        },
    );
}

fn hue_range_sort_benchmark(c: &mut Criterion) {
    const HUE_THRESHOLD_LOW: f32 = 30f32;
    const HUE_THRESHOLD_HIGH: f32 = 210f32;

    let test_images = TestImages::generate();

    c.bench_with_input(
        BenchmarkId::new(
            "hue range sorting, horizontal ascending (512x512, half hue range)",
            1,
        ),
        &test_images,
        |bencher, input| {
            bencher.iter_batched(
                || input.to_owned(),
                |input| {
                    for image in input.into_images() {
                        perform_pixel_sort(
                            image,
                            ImmediateSegmentSelectionMode::HueRange {
                                low: HUE_THRESHOLD_LOW,
                                high: HUE_THRESHOLD_HIGH,
                            },
                            PixelSortOptions {
                                direction: ImageSortingDirection::Horizontal(
                                    PixelSegmentSortDirection::Ascending,
                                ),
                            },
                        );
                    }
                },
                BatchSize::SmallInput,
            );
        },
    );

    c.bench_with_input(
        BenchmarkId::new(
            "hue range sorting, vertical ascending (512x512, half hue range)",
            1,
        ),
        &test_images,
        |bencher, input| {
            bencher.iter_batched(
                || input.to_owned(),
                |input| {
                    for image in input.into_images() {
                        perform_pixel_sort(
                            image,
                            ImmediateSegmentSelectionMode::HueRange {
                                low: HUE_THRESHOLD_LOW,
                                high: HUE_THRESHOLD_HIGH,
                            },
                            PixelSortOptions {
                                direction: ImageSortingDirection::Vertical(
                                    PixelSegmentSortDirection::Ascending,
                                ),
                            },
                        );
                    }
                },
                BatchSize::SmallInput,
            );
        },
    );
}

fn saturation_range_sort_benchmark(c: &mut Criterion) {
    const SATURATION_THRESHOLD_LOW: f32 = 0.15;
    const SATURATION_THRESHOLD_HIGH: f32 = 0.85;

    let test_images = TestImages::generate();

    c.bench_with_input(
        BenchmarkId::new(
            "saturation range sorting, horizontal ascending (512x512, 2/3 saturation range)",
            1,
        ),
        &test_images,
        |bencher, input| {
            bencher.iter_batched(
                || input.to_owned(),
                |input| {
                    for image in input.into_images() {
                        perform_pixel_sort(
                            image,
                            ImmediateSegmentSelectionMode::SaturationRange {
                                low: SATURATION_THRESHOLD_LOW,
                                high: SATURATION_THRESHOLD_HIGH,
                            },
                            PixelSortOptions {
                                direction: ImageSortingDirection::Horizontal(
                                    PixelSegmentSortDirection::Ascending,
                                ),
                            },
                        );
                    }
                },
                BatchSize::SmallInput,
            );
        },
    );

    c.bench_with_input(
        BenchmarkId::new(
            "saturation range sorting, vertical ascending (512x512, 2/3 saturation range)",
            1,
        ),
        &test_images,
        |bencher, input| {
            bencher.iter_batched(
                || input.to_owned(),
                |input| {
                    for image in input.into_images() {
                        perform_pixel_sort(
                            image,
                            ImmediateSegmentSelectionMode::SaturationRange {
                                low: SATURATION_THRESHOLD_LOW,
                                high: SATURATION_THRESHOLD_HIGH,
                            },
                            PixelSortOptions {
                                direction: ImageSortingDirection::Vertical(
                                    PixelSegmentSortDirection::Ascending,
                                ),
                            },
                        );
                    }
                },
                BatchSize::SmallInput,
            );
        },
    );
}

criterion_group! {
    name = benches;
    config =
        Criterion::default()
            .measurement_time(Duration::from_secs(10))
            .sample_size(200);
    targets = luminance_range_sort_benchmark, hue_range_sort_benchmark, saturation_range_sort_benchmark
}

criterion_main!(benches);
