Naive implementation:
```rs
perform_pixel_sort (512x512, half range)/1
                        time:   [1.4926 s 1.4934 s 1.4942 s]
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high mild
```

Rayon per-row implementation:
```rs
perform_pixel_sort (512x512, half range)/1
                        time:   [4.0713 ms 4.0994 ms 4.1309 ms]
                        change: [−99.728% −99.725% −99.723%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 12 outliers among 100 measurements (12.00%)
  7 (7.00%) high mild
  5 (5.00%) high severe
```

Dynamic implementation, but still hardcoded luminance range impl:
```rs
perform_pixel_sort (512x512, half range)/1
                        time:   [4.2397 ms 4.3486 ms 4.4968 ms]
                        change: [+3.3162% +6.0792% +9.5942%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 7 outliers among 100 measurements (7.00%)
  3 (3.00%) high mild
  4 (4.00%) high severe
```

Fully generic implementation:
```rs
perform_pixel_sort (512x512, half range)/1
                        time:   [4.0660 ms 4.0942 ms 4.1238 ms]
                        change: [−3.8948% −2.7432% −1.6350%] (p = 0.00 < 0.05)
                        Performance has improved.
```


More features, now testing on eight images instead of one (divide these times by 8 to get a comparable result):
```rs
luminance range sorting, horizontal ascending (512x512, 2/3 luma range)/1
                        time:   [33.709 ms 33.924 ms 34.151 ms]
Found 16 outliers among 100 measurements (16.00%)
  11 (11.00%) high mild
  5 (5.00%) high severe

Benchmarking luminance range sorting, vertical ascending (512x512, 2/3 luma range)/1: Warming up for 3.0000 s
Warning: Unable to complete 100 samples in 5.0s. You may wish to increase target time to 5.3s, or reduce sample count to 90.
luminance range sorting, vertical ascending (512x512, 2/3 luma range)/1
                        time:   [48.715 ms 49.124 ms 49.571 ms]
Found 16 outliers among 100 measurements (16.00%)
  8 (8.00%) high mild
  8 (8.00%) high severe

hue range sorting, horizontal ascending (512x512, half hue range)/1
                        time:   [34.771 ms 34.971 ms 35.188 ms]
Found 7 outliers among 100 measurements (7.00%)
  6 (6.00%) high mild
  1 (1.00%) high severe

Benchmarking hue range sorting, vertical ascending (512x512, half hue range)/1: Warming up for 3.0000 s
Warning: Unable to complete 100 samples in 5.0s. You may wish to increase target time to 5.3s, or reduce sample count to 90.
hue range sorting, vertical ascending (512x512, half hue range)/1
                        time:   [51.637 ms 52.135 ms 52.650 ms]
Found 3 outliers among 100 measurements (3.00%)
  3 (3.00%) high mild

saturation range sorting, horizontal ascending (512x512, 2/3 saturation range)/1
                        time:   [32.686 ms 32.898 ms 33.133 ms]
Found 18 outliers among 100 measurements (18.00%)
  12 (12.00%) high mild
  6 (6.00%) high severe

Benchmarking saturation range sorting, vertical ascending (512x512, 2/3 saturation range)/1: Warming up for 3.0000 s
Warning: Unable to complete 100 samples in 5.0s. You may wish to increase target time to 5.1s, or reduce sample count to 90.
saturation range sorting, vertical ascending (512x512, 2/3 saturation range)/1
                        time:   [47.811 ms 48.163 ms 48.554 ms]
Found 6 outliers among 100 measurements (6.00%)
  2 (2.00%) high mild
  4 (4.00%) high severe

```


Same, but with more samples and longer benchmarks:
```rs
luminance range sorting, horizontal ascending (512x512, 2/3 luma range)/1
                        time:   [33.783 ms 33.915 ms 34.057 ms]
                        change: [−0.8184% −0.0258% +0.7288%] (p = 0.93 > 0.05)
                        No change in performance detected.
Found 20 outliers among 200 measurements (10.00%)
  19 (9.50%) high mild
  1 (0.50%) high severe

Benchmarking luminance range sorting, vertical ascending (512x512, 2/3 luma range)/1: Warming up for 3.0000 s
Warning: Unable to complete 200 samples in 10.0s. You may wish to increase target time to 10.3s, or reduce sample count to 190.
luminance range sorting, vertical ascending (512x512, 2/3 luma range)/1
                        time:   [48.021 ms 48.225 ms 48.460 ms]
                        change: [−2.8494% −1.8289% −0.9196%] (p = 0.00 < 0.05)
                        Change within noise threshold.
Found 22 outliers among 200 measurements (11.00%)
  17 (8.50%) high mild
  5 (2.50%) high severe

hue range sorting, horizontal ascending (512x512, half hue range)/1
                        time:   [34.895 ms 35.060 ms 35.235 ms]
                        change: [−0.4963% +0.2569% +0.9874%] (p = 0.53 > 0.05)
                        No change in performance detected.
Found 14 outliers among 200 measurements (7.00%)
  11 (5.50%) high mild
  3 (1.50%) high severe

Benchmarking hue range sorting, vertical ascending (512x512, half hue range)/1: Warming up for 3.0000 s
Warning: Unable to complete 200 samples in 10.0s. You may wish to increase target time to 10.6s, or reduce sample count to 180.
hue range sorting, vertical ascending (512x512, half hue range)/1
                        time:   [49.828 ms 50.059 ms 50.306 ms]
                        change: [−5.0292% −3.9819% −2.9694%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 12 outliers among 200 measurements (6.00%)
  8 (4.00%) high mild
  4 (2.00%) high severe

saturation range sorting, horizontal ascending (512x512, 2/3 saturation range)/1
                        time:   [32.649 ms 32.794 ms 32.950 ms]
                        change: [−1.1525% −0.3143% +0.4823%] (p = 0.44 > 0.05)
                        No change in performance detected.
Found 19 outliers among 200 measurements (9.50%)
  13 (6.50%) high mild
  6 (3.00%) high severe

Benchmarking saturation range sorting, vertical ascending (512x512, 2/3 saturation range)/1: Warming up for 3.0000 s
Warning: Unable to complete 200 samples in 10.0s. You may wish to increase target time to 10.3s, or reduce sample count to 190.
saturation range sorting, vertical ascending (512x512, 2/3 saturation range)/1
                        time:   [47.555 ms 47.783 ms 48.047 ms]
                        change: [−1.7084% −0.7891% +0.1352%] (p = 0.07 > 0.05)
                        No change in performance detected.
Found 19 outliers among 200 measurements (9.50%)
  11 (5.50%) high mild
  8 (4.00%) high severe
```

After direct gamma to linear f32 conversion:
```rs
luminance range sorting, horizontal ascending (512x512, 2/3 luma range)/1
                        time:   [32.024 ms 32.282 ms 32.603 ms]
                        change: [−5.6427% −4.8170% −3.7916%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 11 outliers among 200 measurements (5.50%)
  7 (3.50%) high mild
  4 (2.00%) high severe

Benchmarking luminance range sorting, vertical ascending (512x512, 2/3 luma range)/1: Warming up for 3.0000 s
Warning: Unable to complete 200 samples in 10.0s. You may wish to increase target time to 10.1s, or reduce sample count to 190.
luminance range sorting, vertical ascending (512x512, 2/3 luma range)/1
                        time:   [46.393 ms 46.670 ms 46.977 ms]
                        change: [−3.9138% −3.2249% −2.4541%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 22 outliers among 200 measurements (11.00%)
  8 (4.00%) high mild
  14 (7.00%) high severe

hue range sorting, horizontal ascending (512x512, half hue range)/1
                        time:   [32.900 ms 33.192 ms 33.523 ms]
                        change: [−6.2857% −5.3277% −4.1262%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 28 outliers among 200 measurements (14.00%)
  13 (6.50%) high mild
  15 (7.50%) high severe

Benchmarking hue range sorting, vertical ascending (512x512, half hue range)/1: Warming up for 3.0000 s
Warning: Unable to complete 200 samples in 10.0s. You may wish to increase target time to 10.2s, or reduce sample count to 190.
hue range sorting, vertical ascending (512x512, half hue range)/1
                        time:   [47.732 ms 47.933 ms 48.149 ms]
                        change: [−4.8638% −4.2470% −3.6370%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 13 outliers among 200 measurements (6.50%)
  9 (4.50%) high mild
  4 (2.00%) high severe

saturation range sorting, horizontal ascending (512x512, 2/3 saturation range)/1
                        time:   [31.666 ms 31.816 ms 31.979 ms]
                        change: [−3.6370% −2.9828% −2.3581%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 20 outliers among 200 measurements (10.00%)
  19 (9.50%) high mild
  1 (0.50%) high severe

saturation range sorting, vertical ascending (512x512, 2/3 saturation range)/1
                        time:   [46.541 ms 46.783 ms 47.087 ms]
                        change: [−2.8172% −2.0927% −1.2987%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 9 outliers among 200 measurements (4.50%)
  6 (3.00%) high mild
  3 (1.50%) high severe
```

After lto = fat and 1 codegen unit:
```rs
luminance range sorting, horizontal ascending (512x512, 2/3 luma range)/1
                        time:   [31.757 ms 31.919 ms 32.093 ms]
                        change: [−2.1808% −1.1220% −0.1223%] (p = 0.03 < 0.05)
                        Change within noise threshold.
Found 18 outliers among 200 measurements (9.00%)
  16 (8.00%) high mild
  2 (1.00%) high severe

Benchmarking luminance range sorting, vertical ascending (512x512, 2/3 luma range)/1: Warming up for 3.0000 s
Warning: Unable to complete 200 samples in 10.0s. You may wish to increase target time to 10.0s, or reduce sample count to 190.
luminance range sorting, vertical ascending (512x512, 2/3 luma range)/1
                        time:   [48.140 ms 48.486 ms 48.853 ms]
                        change: [+2.9051% +3.8916% +4.8774%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 6 outliers among 200 measurements (3.00%)
  4 (2.00%) high mild
  2 (1.00%) high severe

hue range sorting, horizontal ascending (512x512, half hue range)/1
                        time:   [32.337 ms 32.474 ms 32.618 ms]
                        change: [−3.2296% −2.1655% −1.2022%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 9 outliers among 200 measurements (4.50%)
  8 (4.00%) high mild
  1 (0.50%) high severe

Benchmarking hue range sorting, vertical ascending (512x512, half hue range)/1: Warming up for 3.0000 s
Warning: Unable to complete 200 samples in 10.0s. You may wish to increase target time to 10.3s, or reduce sample count to 190.
hue range sorting, vertical ascending (512x512, half hue range)/1
                        time:   [48.376 ms 48.710 ms 49.081 ms]
                        change: [+0.8270% +1.6209% +2.4501%] (p = 0.00 < 0.05)
                        Change within noise threshold.
Found 14 outliers among 200 measurements (7.00%)
  8 (4.00%) high mild
  6 (3.00%) high severe

saturation range sorting, horizontal ascending (512x512, 2/3 saturation range)/1
                        time:   [31.370 ms 31.566 ms 31.772 ms]
                        change: [−1.5851% −0.7870% +0.0003%] (p = 0.06 > 0.05)
                        No change in performance detected.
Found 4 outliers among 200 measurements (2.00%)
  4 (2.00%) high mild

saturation range sorting, vertical ascending (512x512, 2/3 saturation range)/1
                        time:   [46.549 ms 46.695 ms 46.853 ms]
                        change: [−0.9021% −0.1881% +0.4435%] (p = 0.61 > 0.05)
                        No change in performance detected.
Found 11 outliers among 200 measurements (5.50%)
  8 (4.00%) high mild
  3 (1.50%) high severe
```

After target-cpu=native:
```rs
luminance range sorting, horizontal ascending (512x512, 2/3 luma range)/1
                        time:   [31.305 ms 31.475 ms 31.659 ms]
                        change: [−2.1635% −1.3915% −0.6529%] (p = 0.00 < 0.05)
                        Change within noise threshold.
Found 9 outliers among 200 measurements (4.50%)
  7 (3.50%) high mild
  2 (1.00%) high severe

luminance range sorting, vertical ascending (512x512, 2/3 luma range)/1
                        time:   [46.861 ms 46.995 ms 47.138 ms]
                        change: [−3.8500% −3.0748% −2.3213%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 13 outliers among 200 measurements (6.50%)
  11 (5.50%) high mild
  2 (1.00%) high severe

hue range sorting, horizontal ascending (512x512, half hue range)/1
                        time:   [36.558 ms 36.702 ms 36.853 ms]
                        change: [+12.339% +13.022% +13.719%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 12 outliers among 200 measurements (6.00%)
  12 (6.00%) high mild

Benchmarking hue range sorting, vertical ascending (512x512, half hue range)/1: Warming up for 3.0000 s
Warning: Unable to complete 200 samples in 10.0s. You may wish to increase target time to 11.1s, or reduce sample count to 180.
hue range sorting, vertical ascending (512x512, half hue range)/1
                        time:   [52.808 ms 53.043 ms 53.303 ms]
                        change: [+7.9008% +8.8956% +9.8745%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 11 outliers among 200 measurements (5.50%)
  8 (4.00%) high mild
  3 (1.50%) high severe

saturation range sorting, horizontal ascending (512x512, 2/3 saturation range)/1
                        time:   [34.376 ms 34.530 ms 34.697 ms]
                        change: [+8.5101% +9.3904% +10.240%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 17 outliers among 200 measurements (8.50%)
  14 (7.00%) high mild
  3 (1.50%) high severe

Benchmarking saturation range sorting, vertical ascending (512x512, 2/3 saturation range)/1: Warming up for 3.0000 s
Warning: Unable to complete 200 samples in 10.0s. You may wish to increase target time to 10.7s, or reduce sample count to 180.
saturation range sorting, vertical ascending (512x512, 2/3 saturation range)/1
                        time:   [51.061 ms 51.402 ms 51.764 ms]
                        change: [+9.2444% +10.081% +11.048%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 11 outliers among 200 measurements (5.50%)
  11 (5.50%) high mild
```

