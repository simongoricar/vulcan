use criterion::{
    BatchSize,
    BenchmarkId,
    Criterion,
    criterion_group,
    criterion_main,
};
use image::{Rgba, RgbaImage};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use vulcan_core::sorting::{
    ImageSortingDirection,
    PixelSegmentSelectionMode,
    PixelSegmentSortDirection,
    PixelSortOptions,
    perform_pixel_sort,
};

fn prepare_input_image() -> RgbaImage {
    const SAMPLE_IMAGE_WIDTH: u32 = 512;
    const SAMPLE_IMAGE_HEIGHT: u32 = 512;

    const SAMPLE_IMAGE_SEED: u64 = 205536008065667967u64;

    let mut generator = ChaCha8Rng::seed_from_u64(SAMPLE_IMAGE_SEED);

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

fn horizontal_sort_benchmark(c: &mut Criterion) {
    let sample_image = prepare_input_image();

    c.bench_with_input(
        BenchmarkId::new("perform_pixel_sort (512x512, half range)", 1),
        &sample_image,
        |bencher, input| {
            bencher.iter_batched(
                || input.to_owned(),
                |input| {
                    perform_pixel_sort(
                        input,
                        PixelSegmentSelectionMode::LuminanceRange {
                            low: 0.15f32,
                            high: 0.85f32,
                        },
                        PixelSortOptions {
                            direction: ImageSortingDirection::Horizontal(
                                PixelSegmentSortDirection::Ascending,
                            ),
                        },
                    )
                },
                BatchSize::SmallInput,
            );
        },
    );
}

criterion_group!(benches, horizontal_sort_benchmark);
criterion_main!(benches);
