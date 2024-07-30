use std::{collections::HashSet, fs};

use xcap::image::{self, GenericImageView, Rgba, RgbaImage};

const TILE_SIZE: u32 = 33;

#[allow(dead_code)]
pub fn tile_bg_colors() {
    let tiles_img = image::open(BACKGROUND_SPRITE_PATH).unwrap();
    let supermask_img = image::open("supermask.png").unwrap();
    let mut out_img = RgbaImage::new(tiles_img.width(), tiles_img.height());

    for ty in 0..8 {
        for tx in 0..8 {
            for y in 0..32 {
                for x in 0..32 {
                    let src_x = tx * TILE_SIZE + x;
                    let src_y = ty * TILE_SIZE + y + 1;
                    let dst_x = tx + 8 * x;
                    let dst_y = ty + 8 * y;
                    let mut pixel = tiles_img.get_pixel(src_x, src_y);
                    let darken = 1;
                    if supermask_img.get_pixel(x, y).0[0] == 0 {
                        pixel.0[0] /= darken;
                        pixel.0[1] /= darken;
                        pixel.0[2] /= darken;
                        // pixel.0[3] /= 2;
                    }
                    out_img.put_pixel(dst_x, dst_y, pixel);
                }
            }
        }
    }
    out_img.save("collated.png").unwrap();
}

const SPRITE_PATH: &str = "tokyo/resized";
const TREASURE_SPRITE_PATH: &str = "treasure.png";
const BACKGROUND_SPRITE_PATH: &str = "Content/Packed/textures/tokyo/tiles_grid.png";
#[allow(dead_code)]
pub fn find_monster_sample_offset() {
    let background_img = image::open(BACKGROUND_SPRITE_PATH).unwrap();
    let treasure_img = image::open(TREASURE_SPRITE_PATH).unwrap();
    let mut background_pixels: [[HashSet<Rgba<u8>>; 32]; 32] = Default::default();
    let mut monster_pixels: [[HashSet<Rgba<u8>>; 32]; 32] = Default::default();
    let mut valid_sample: [[bool; 32]; 32] = [[true; 32]; 32];

    for y in 0..32 {
        for x in 0..32 {
            for ty in 0..8 {
                for tx in 0..8 {
                    let src_x = tx * TILE_SIZE + x;
                    let src_y = ty * TILE_SIZE + y + 1;
                    background_pixels[y as usize][x as usize]
                        .insert(background_img.get_pixel(src_x, src_y));
                }
            }
        }
    }

    println!("background pixel counts");
    for y in 0..32 {
        print!("  ");
        for x in 0..32 {
            print!("{: >4}", background_pixels[y][x].len());
        }
        println!();
    }

    let mut frame_count = 0;
    for monster in fs::read_dir(SPRITE_PATH).unwrap() {
        let monster = monster.unwrap();
        for frame in fs::read_dir(monster.path()).unwrap() {
            let frame = frame.unwrap();
            let monster_img = image::open(frame.path()).unwrap();
            for y in 0..32 {
                for x in 0..32 {
                    let p = monster_img.get_pixel(x as u32, y as u32);
                    if p.0[3] < 255
                        || p == treasure_img.get_pixel(x as u32, y as u32)
                        || background_pixels[y][x].contains(&p)
                    {
                        valid_sample[y][x] = false;
                    } else {
                        monster_pixels[y][x].insert(p);
                    }
                }
            }
            frame_count += 1;
        }
    }

    println!("monster pixel counts: ({frame_count} frames)");
    for y in 0..32 {
        print!("  ");
        for x in 0..32 {
            print!("{: >4}", monster_pixels[y][x].len());
        }
        println!();
    }

    println!("valid samples pixel counts: ({frame_count} frames)");
    for y in 0..32 {
        print!("  ");
        for x in 0..32 {
            print!("{: >6}", valid_sample[y][x]);
        }
        println!();
    }
}

#[allow(dead_code)]
pub fn print_background_pixels() {
    let background_img = image::open(BACKGROUND_SPRITE_PATH).unwrap();
    println!("const BACKGROUND_PIXELS: [[[u8; 4]; 8]; 8] = [");
    for y in 0..8 {
        print!("    [");
        for x in 0..8 {
            let src_x = x * 33 + 16;
            let src_y = y * 33 + 12 + 1;
            print!("{:?}, ", background_img.get_pixel(src_x, src_y).0);
        }
        println!("],");
    }
    println!("];");
}

const LARGE_FONT_PATH: &str = "tokyo/fonts/numbers.png";
const DIGIT_OFFSETS: &[u32] = &[4, 32, 60, 88, 116, 144, 172, 200, 228];
const MAX_PATTERN_WIDTH: u32 = 22;
const MAX_PATTERN_HEIGHT: u32 = 16;
#[allow(dead_code)]
pub fn get_large_digit_discriminant() {
    let digits_img = image::open(LARGE_FONT_PATH).unwrap();

    // Extract individual digits (brown 0, red 1-7)
    let digits_imgs = [0, 1, 2, 3, 4, 5, 6, 7].map(|i| {
        digits_img.view(
            DIGIT_OFFSETS[i],
            if i == 0 { 32 } else { 0 },
            MAX_PATTERN_WIDTH,
            MAX_PATTERN_HEIGHT,
        )
    });

    for (i, digit) in digits_imgs.iter().enumerate() {
        digit
            .to_image()
            .save(format!("script_output/digit_discrim/{i}.png"))
            .unwrap();
    }
    // Create a window with size 'len' and move it across each digit
    let mut best_size = u32::MAX;
    let mut k = 0;
    // Iterate over windows of varying sizes
    for width in (1..=MAX_PATTERN_WIDTH).rev() {
        for height in (1..=MAX_PATTERN_HEIGHT).rev() {
            if width * height < 8 {
                continue;
            }
            for y in 0..=MAX_PATTERN_HEIGHT - height {
                for x in 0..=MAX_PATTERN_WIDTH - width {
                    // Count digits in red numbers that pass threshold filter
                    let counts = [0, 1, 2, 3, 4, 5, 6, 7].map(|i| {
                        digits_imgs[i]
                            .view(x, y, width, height)
                            .pixels()
                            .map(|(_, _, p)| p.0)
                            .filter(|p| p[1] == 91 && p[2] >= 69 && p[2] <= 78)
                            .count()
                    });
                    // Check that the counts are unique
                    let unique_count = HashSet::from(counts).len();
                    if unique_count >= 8 {
                        if width * height < best_size {
                            best_size = width * height;
                            println!("{width}x{height}+{x}+{y}");
                            // println!("  {counts:?}");
                            // create image for blog
                            let mut img =
                                RgbaImage::new(MAX_PATTERN_WIDTH * 8 + 8, MAX_PATTERN_HEIGHT);
                            for (i, digit) in digits_imgs.iter().enumerate() {
                                for (xx, yy, p) in digit.pixels() {
                                    let col = if p.0[1] == 91 && p.0[2] >= 69 && p.0[2] <= 78 {
                                        [255, 255, 255, 255]
                                    } else {
                                        [0, 0, 0, 0]
                                    };
                                    img.get_pixel_mut(
                                        xx + i as u32 * MAX_PATTERN_WIDTH + i as u32,
                                        yy,
                                    )
                                    .0 = col;
                                    crate::util::draw_rect(
                                        &mut img,
                                        x + i as u32 * MAX_PATTERN_WIDTH + i as u32,
                                        y,
                                        width,
                                        height,
                                        [0, 128, 255, 64],
                                    );
                                }
                            }
                            let save_amount = if k == 32 { 10 } else { 1 };
                            for _ in 0..save_amount {
                                img.save(format!("script_output/digit_discrim/full_{k:02}.webp"))
                                    .unwrap();
                                k += 1;
                            }
                        }
                    }
                }
            }
        }
    }
}
