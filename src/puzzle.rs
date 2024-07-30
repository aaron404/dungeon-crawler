use std::{
    fmt::{Display, Write},
    io::{Cursor, Read},
};

use byteorder::{ReadBytesExt, LE};
use thiserror::Error;
use xcap::{
    image::{open, DynamicImage, GenericImageView, Rgba, RgbaImage, SubImage},
    XCapError,
};

use anyhow::anyhow;
use anyhow::Result;

use crate::util::draw_rect;

const TILE_STRIDE: u32 = 33;
const TILE_SIZE: u32 = 32;
const TILE_SAMPLE_POINT: (u32, u32) = (16, 12);

const BOARD_BASE: (u32, u32) = (49, 175);
const BOARD_SIZE: (u32, u32) = (264, 265);

const TOP_NUMS_BASE: (u32, u32) = (55, 138);
const TOP_NUMS_SIZE: (u32, u32) = (263, 32);
const TOP_NUMS_OFFSETS: [u32; 8] = [1, 0, 0, 0, 0, 0, 0, 0];

const LEFT_NUMS_BASE: (u32, u32) = (19, 174);
const LEFT_NUMS_SIZE: (u32, u32) = (32, 263);
const LEFT_NUMS_OFFSETS: [u32; 8] = [0, 2, 2, 1, 1, 2, 2, 1];

const SEED_BASE: (u32, u32) = (109, 103);
const SEED_SIZE: (u32, u32) = (63, 7);
const SEED_OFFSETS: [u32; 10] = [8, 5, 8, 8, 7, 8, 8, 8, 8, 8];

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("xcap")]
    XCapError(#[from] XCapError),
}

#[rustfmt::skip]
const BACKGROUND_PIXELS: [[[u8; 4]; 8]; 8] = [
    [[176, 128, 93, 255], [55, 58, 59, 255], [125, 113, 90, 255], [54, 58, 55, 255], [176, 128, 93, 255], [55, 58, 59, 255], [125, 113, 90, 255], [54, 58, 55, 255], ],
    [[ 55,  58, 59, 255], [56, 59, 55, 255], [ 54,  58, 55, 255], [54, 58, 55, 255], [55, 58, 59, 255], [55, 58, 59, 255], [54, 58, 55, 255], [54, 58, 55, 255], ],
    [[ 57,  59, 55, 255], [55, 58, 59, 255], [ 54,  58, 55, 255], [54, 56, 58, 255], [54, 58, 55, 255], [54, 58, 55, 255], [57, 59, 55, 255], [55, 58, 59, 255], ],
    [[ 55,  58, 59, 255], [55, 58, 59, 255], [ 54,  58, 55, 255], [54, 58, 55, 255], [54, 58, 55, 255], [54, 58, 55, 255], [55, 58, 59, 255], [55, 58, 59, 255], ],
    [[ 54,  58, 55, 255], [54, 58, 55, 255], [ 57,  59, 55, 255], [55, 58, 59, 255], [54, 58, 55, 255], [54, 56, 58, 255], [54, 58, 55, 255], [40, 44, 41, 255], ],
    [[ 54,  58, 55, 255], [54, 58, 55, 255], [ 55,  58, 59, 255], [55, 58, 59, 255], [54, 58, 55, 255], [54, 58, 55, 255], [54, 58, 55, 255], [54, 58, 55, 255], ],
    [[ 57,  59, 55, 255], [55, 58, 59, 255], [ 54,  58, 55, 255], [54, 56, 58, 255], [54, 58, 55, 255], [54, 58, 55, 255], [57, 59, 55, 255], [55, 58, 59, 255], ],
    [[ 55,  58, 59, 255], [55, 58, 59, 255], [ 54,  58, 55, 255], [54, 58, 55, 255], [54, 58, 55, 255], [54, 58, 55, 255], [55, 58, 59, 255], [55, 58, 59, 255], ],
];
const TREASURE_COLOR: [u8; 4] = [220, 170, 109, 255];

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub enum Tile {
    #[default]
    Empty,
    Wall,
    Treasure,
    Monster,
}

impl Tile {
    const fn to_str(&self) -> &str {
        match self {
            Tile::Empty => "•",
            Tile::Wall => "W",
            Tile::Treasure => "T",
            Tile::Monster => "M",
        }
    }

    pub fn is_monster(self) -> u64 {
        if self == Tile::Monster {
            1
        } else {
            0
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Puzzle {
    pub tiles: [[Tile; 8]; 8],
    pub top_nums: [u8; 8],
    pub left_nums: [u8; 8],
    pub seed: Option<u32>,
}

impl Puzzle {
    pub fn from_image(img: SubImage<&RgbaImage>) -> Result<Self> {
        // Crop regions for the board, seed, and wall counts
        let tiles = img.view(BOARD_BASE.0, BOARD_BASE.1, BOARD_SIZE.0, BOARD_SIZE.1);
        let top_nums = img.view(
            TOP_NUMS_BASE.0,
            TOP_NUMS_BASE.1,
            TOP_NUMS_SIZE.0,
            TOP_NUMS_SIZE.1,
        );
        let left_nums = img.view(
            LEFT_NUMS_BASE.0,
            LEFT_NUMS_BASE.1,
            LEFT_NUMS_SIZE.0,
            LEFT_NUMS_SIZE.1,
        );
        let seed = img.view(SEED_BASE.0, SEED_BASE.1, SEED_SIZE.0, SEED_SIZE.1);

        #[cfg(debug_assertions)]
        {
            tiles
                .to_image()
                .save("script_output/segmentation/tiles_img.png")?;
            top_nums
                .to_image()
                .save("script_output/segmentation/top_nums_img.png")?;
            left_nums
                .to_image()
                .save("script_output/segmentation/left_nums_img.png")?;
            seed.to_image()
                .save("script_output/segmentation/seed.png")?;

            let mut img = img.to_image();
            draw_rect(
                &mut img,
                BOARD_BASE.0,
                BOARD_BASE.1,
                BOARD_SIZE.0,
                BOARD_SIZE.1,
                [255, 0, 0, 128],
            );

            draw_rect(
                &mut img,
                TOP_NUMS_BASE.0,
                TOP_NUMS_BASE.1,
                TOP_NUMS_SIZE.0,
                TOP_NUMS_SIZE.1,
                [0, 128, 255, 128],
            );

            draw_rect(
                &mut img,
                LEFT_NUMS_BASE.0,
                LEFT_NUMS_BASE.1,
                LEFT_NUMS_SIZE.0,
                LEFT_NUMS_SIZE.1,
                [0, 128, 255, 128],
            );

            draw_rect(
                &mut img,
                SEED_BASE.0,
                SEED_BASE.1,
                SEED_SIZE.0,
                SEED_SIZE.1,
                [0, 255, 0, 128],
            );

            img.save("script_output/segmentation/overlay.png")?;
        }

        let tiles = parse_tiles(tiles);
        let (top_nums, left_nums) = parse_wall_counts(top_nums, left_nums)?;
        let seed = parse_seed(seed)?;

        Ok(Self {
            tiles,
            top_nums,
            left_nums,
            seed,
        })
    }

    pub fn draw_parsing_overlay(&self, img: SubImage<&RgbaImage>) {
        // Create mutable clone of the image view
        let mut img = img.to_image();

        #[cfg(debug_assertions)]
        let mut hash: u64 = 0;
        for tile_y in 0..8 {
            for tile_x in 0..8 {
                #[cfg(debug_assertions)]
                let hash_bit = tile_y * 8 + tile_x;
                let col = match self.tiles[tile_y][tile_x] {
                    Tile::Treasure => [0, 255, 0, 64],
                    Tile::Monster => [255, 0, 0, 64],
                    _ => [0, 0, 0, 0],
                };

                #[cfg(debug_assertions)]
                if col != [0, 0, 0, 0] {
                    hash |= 1 << hash_bit;
                }

                draw_rect(
                    &mut img,
                    tile_x as u32 * TILE_STRIDE + BOARD_BASE.0,
                    tile_y as u32 * TILE_STRIDE + BOARD_BASE.1,
                    TILE_SIZE,
                    TILE_SIZE,
                    col,
                );
            }
        }

        const DIGIT_OFFSETS: &[u32] = &[4, 32, 60, 88, 116, 144, 172, 200, 228];
        let nums_overlay = open("nums_overlay.png").unwrap();

        for i in 0..8 {
            let src_x = DIGIT_OFFSETS[self.top_nums[i] as usize];
            let dst_x = TOP_NUMS_BASE.0 + TOP_NUMS_OFFSETS[i] + i as u32 * TILE_STRIDE;
            let dst_y = TOP_NUMS_BASE.1;
            overlay_img(&nums_overlay, &mut img, src_x, 0, dst_x, dst_y, 24, 28);

            let src_x = DIGIT_OFFSETS[self.left_nums[i] as usize];
            let dst_x = LEFT_NUMS_BASE.0;
            let dst_y = LEFT_NUMS_BASE.1 + LEFT_NUMS_OFFSETS[i] + i as u32 * TILE_STRIDE;
            overlay_img(&nums_overlay, &mut img, src_x, 0, dst_x, dst_y, 24, 28);
        }

        #[cfg(debug_assertions)]
        img.save(format!("script_output/parsing/{hash}.png"))
            .unwrap();
    }

    // The generic parameter T lets me serialize to a buffer or directly to a file
    pub fn serialize<T>(&self, cursor: &mut T) -> Result<()>
    where
        T: std::io::Write + byteorder::WriteBytesExt,
    {
        // Concatenate top and left numbers, fold them 3 bits at a time into a u64
        let wall_counts: u64 = [self.top_nums, self.left_nums]
            .concat()
            .iter()
            .fold(0, |acc, &x| (acc << 3) + x.min(7) as u64);

        // Iterate over each tile, mapping each monster to 1, else 0. Fold into a u64
        let monster_locations = self
            .tiles
            .iter()
            .flatten()
            .map(|tile| if let Tile::Monster = tile { 1 } else { 0 })
            .fold(0, |acc, x| (acc << 1) + x as u64);

        // Repeat for treasure locations
        let treasure_locations = self
            .tiles
            .iter()
            .flatten()
            .map(|tile| if let Tile::Treasure = tile { 1 } else { 0 })
            .fold(0, |acc, x| (acc << 1) + x as u64);

        // Seed is 0 for None or (seed + 1) for Some
        if let Some(seed) = self.seed {
            cursor.write_u32::<LE>(seed + 1)?;
        } else {
            cursor.write_u32::<LE>(0)?;
        }
        cursor.write_all(&wall_counts.to_le_bytes()[0..6])?;
        cursor.write_u64::<LE>(monster_locations)?;
        cursor.write_u64::<LE>(treasure_locations)?;

        Ok(())
    }

    pub fn deserialize(bytes: &[u8; 26]) -> Result<Self> {
        let mut cursor = Cursor::new(bytes);
        // Read the seed first. Subtract 1 if it was non-zero.
        let seed = cursor.read_u32::<LE>()?;
        let seed = if seed == 0 { None } else { Some(seed - 1) };

        // Read the packed wall counts into an 8 byte buffer and convert to a u64
        let mut wall_counts: [u8; 8] = Default::default();
        cursor.read_exact(&mut wall_counts[0..6])?;
        let wall_counts = u64::from_le_bytes(wall_counts);

        // Decode the top and left counts with reverse shift and mask operations
        let top_nums =
            [0, 1, 2, 3, 4, 5, 6, 7].map(|i| (wall_counts >> (45 - (i * 3))) as u8 & 0b111);
        let left_nums =
            [0, 1, 2, 3, 4, 5, 6, 7].map(|i| (wall_counts >> (21 - (i * 3))) as u8 & 0b111);

        // Read and unpack the monster/treasure locations
        let monster_locations = cursor.read_u64::<LE>()?;
        let treasure_locations = cursor.read_u64::<LE>()?;
        let mut tiles = [[Tile::Empty; 8]; 8];
        for y in 0..8 {
            for x in 0..8 {
                let i = y * 8 + x;
                let monster = monster_locations & (1 << (63 - i)) > 0;
                let treasure = treasure_locations & (1 << (63 - i)) > 0;
                if monster {
                    tiles[y][x] = Tile::Monster;
                } else if treasure {
                    tiles[y][x] = Tile::Treasure;
                }
            }
        }

        Ok(Self {
            tiles,
            top_nums,
            left_nums,
            seed,
        })
    }
}

// Map the discriminant values to the digits they represent
fn count_to_digit(count: usize) -> Result<u8> {
    Ok(match count {
        4 => 0u8,
        0 => 1,
        5 => 2,
        2 => 3,
        3 => 4,
        8 => 5,
        6 => 6,
        1 => 7,
        _ => return Err(anyhow!("Error parsing digit")),
    })
}

fn parse_tiles(img: SubImage<&RgbaImage>) -> [[Tile; 8]; 8] {
    // Input image is a cropped view of only the tiles
    let mut tiles = [[Tile::Empty; 8]; 8];
    for tile_y in 0..8usize {
        for tile_x in 0..8usize {
            // Lookup the background color for the current tile
            let bg_color = BACKGROUND_PIXELS[tile_y][tile_x];
            let px = tile_x as u32 * TILE_STRIDE + TILE_SAMPLE_POINT.0;
            let py = tile_y as u32 * TILE_STRIDE + TILE_SAMPLE_POINT.1;

            // Fetch the color at the tile's sample point
            let sample = img.get_pixel(px, py).0;

            // #computervision
            if sample == bg_color {
                tiles[tile_y][tile_x] = Tile::Empty;
            } else if sample == TREASURE_COLOR {
                tiles[tile_y][tile_x] = Tile::Treasure;
            } else {
                tiles[tile_y][tile_x] = Tile::Monster;
            }
        }
    }

    tiles
}
// Pass in two subimages cropped to the numbers on the top and left sides
fn parse_wall_counts(
    top_img: SubImage<&RgbaImage>,
    left_img: SubImage<&RgbaImage>,
) -> Result<([u8; 8], [u8; 8])> {
    let mut top_nums = [0; 8];
    let mut left_nums = [0; 8];
    for i in 0..8 {
        // Crop the image to a single digit, threshold, and count pixels
        let top_x = i * TILE_STRIDE + 3 + TOP_NUMS_OFFSETS[i as usize];
        let top_y = 11;
        let top_count = top_img
            .view(top_x, top_y, 4, 2)
            .pixels()
            .map(|(_, _, p)| p.0)
            .filter(|&[_r, g, b, _a]| g == 91 && b >= 69 && b <= 78)
            .count();

        // Repeat for the numbers on the left side
        let left_x = 3;
        let left_y = i * TILE_STRIDE + 11 + LEFT_NUMS_OFFSETS[i as usize];
        let left_count = left_img
            .view(left_x, left_y, 4, 2)
            .pixels()
            .map(|(_, _, p)| p.0)
            .filter(|&[_r, g, b, _a]| g == 91 && b >= 69 && b <= 78)
            .count();

        // Map the counts to digits using the discriminant values
        top_nums[i as usize] = count_to_digit(top_count)?;
        left_nums[i as usize] = count_to_digit(left_count)?;
    }

    Ok((top_nums, left_nums))
}
fn parse_seed(img: SubImage<&RgbaImage>) -> Result<Option<u32>> {
    let mut seed = 0u32;
    let mut seed_present = false;
    let mut x = 0;

    // Scan from the left
    while x < SEED_SIZE.0 {
        // Compute the hash
        let hash = img
            .view(x, 0, 1, SEED_SIZE.1)
            .pixels()
            .map(|(_x, _y, Rgba([r, _g, _b, _a]))| if r == 52 { 1 } else { 0 })
            .fold(0, |acc, x| (acc << 1) + x);

        // Column was empty, move to the next column
        if hash == 0 {
            x += 1;
            continue;
        }

        seed_present = true;

        // Map the hash back to a digit
        let digit = match hash {
            28 => 0,
            33 => 1,
            17 => 2,
            18 => 3,
            8 => 4,
            122 => 5,
            62 => 6,
            16 => 7,
            54 => 8,
            50 => 9,
            _ => return Err(anyhow!("Seed parse error")),
        };

        // Keep running tally of the seed as we find each digit
        seed = seed * 10 + digit;
        x += SEED_OFFSETS[digit as usize];
    }

    // If there was no seed, return None (for curated puzzles with no seed)
    Ok(seed_present.then_some(seed))
}

impl Display for Puzzle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(' ')?;
        for i in 0..8 {
            f.write_fmt(format_args!(" {}", self.top_nums[i]))?;
        }

        f.write_char('\n')?;

        for row in 0..8 {
            f.write_fmt(format_args!("{}", self.left_nums[row]))?;
            for i in 0..8 {
                f.write_fmt(format_args!(" {}", self.tiles[row][i].to_str()))?;
            }
            f.write_char('\n')?;
        }

        Ok(())
    }
}

fn overlay_img(
    src: &DynamicImage,
    dst: &mut RgbaImage,
    src_x: u32,
    src_y: u32,
    dst_x: u32,
    dst_y: u32,
    width: u32,
    height: u32,
) {
    for y in 0..height {
        for x in 0..width {
            let src_p = src.get_pixel(x + src_x, y + src_y).0;
            let dst_p = dst.get_pixel_mut(x + dst_x, y + dst_y);
            let src_a = src_p[3] as u32;
            for i in 0..3 {
                dst_p.0[i] =
                    ((src_p[i] as u32 * src_a + dst_p[i] as u32 * (255 - src_a)) / 255) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use test_case::test_case;

    use super::*;

    fn get_reference_puzzle(monster: &str) -> Puzzle {
        use Tile::*;
        match monster {
            "insectoid" => Puzzle {
                tiles: [
                    [Empty, Empty, Empty, Empty, Empty, Empty, Monster, Empty],
                    [Empty, Monster, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Monster, Empty, Empty, Empty, Empty, Monster, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Monster, Empty, Empty, Empty, Empty, Empty, Monster, Empty],
                    [Empty, Empty, Monster, Empty, Empty, Empty, Empty, Empty],
                ],
                top_nums: [6, 4, 5, 2, 3, 6, 1, 7],
                left_nums: [4, 4, 4, 6, 4, 4, 5, 3],
                seed: None,
            },
            "minotaur" => Puzzle {
                tiles: [
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Treasure, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Treasure, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Monster, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                ],
                top_nums: [7, 3, 2, 2, 4, 4, 3, 2],
                left_nums: [5, 2, 2, 4, 4, 5, 2, 3],
                seed: Some(24737362),
            },
            "goblin" => Puzzle {
                tiles: [
                    [Empty, Monster, Empty, Empty, Empty, Monster, Empty, Monster],
                    [Monster, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Monster, Empty, Empty, Treasure, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Monster, Empty, Empty, Empty, Empty, Empty],
                ],
                top_nums: [3, 4, 2, 4, 6, 1, 4, 2],
                left_nums: [4, 2, 3, 3, 3, 1, 4, 6],
                seed: Some(22398633),
            },
            "kobold" => Puzzle {
                tiles: [
                    [Empty, Empty, Empty, Empty, Monster, Empty, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Treasure, Empty, Empty, Empty, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Monster, Empty],
                    [Monster, Empty, Empty, Empty, Empty, Monster, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Monster, Empty, Monster, Empty, Empty, Empty, Empty],
                ],
                top_nums: [4, 1, 4, 2, 5, 3, 3, 4],
                left_nums: [1, 4, 0, 6, 4, 4, 1, 6],
                seed: Some(79019143),
            },
            "imp" => Puzzle {
                tiles: [
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Monster, Empty, Empty, Empty, Empty, Empty, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Monster, Empty, Empty, Empty, Monster, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Treasure, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Monster, Empty, Empty, Monster],
                ],
                top_nums: [4, 2, 4, 2, 4, 6, 0, 6],
                left_nums: [4, 2, 5, 4, 6, 2, 3, 2],
                seed: Some(17827485),
            },
            "skeleton" => Puzzle {
                tiles: [
                    [Empty, Empty, Empty, Empty, Empty, Monster, Empty, Monster],
                    [Empty, Treasure, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Monster, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Monster, Empty, Monster, Empty, Monster],
                ],
                top_nums: [3, 3, 2, 2, 7, 0, 6, 4],
                left_nums: [3, 2, 3, 6, 3, 5, 0, 5],
                seed: Some(29032690),
            },
            "king" => Puzzle {
                tiles: [
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Treasure, Empty, Empty, Empty, Empty, Empty, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Monster],
                    [Empty, Monster, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Monster, Empty, Empty, Empty],
                ],
                top_nums: [0, 3, 2, 7, 2, 3, 3, 4],
                left_nums: [1, 3, 3, 5, 2, 3, 5, 2],
                seed: Some(51114261),
            },
            "lich" => Puzzle {
                tiles: [
                    [Empty, Empty, Empty, Empty, Empty, Monster, Empty, Monster],
                    [Empty, Treasure, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Monster, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Monster, Empty, Monster, Empty, Monster],
                ],
                top_nums: [3, 3, 2, 2, 7, 0, 6, 4],
                left_nums: [3, 2, 3, 6, 3, 5, 0, 5],
                seed: Some(29032690),
            },
            "goat" => Puzzle {
                tiles: [
                    [Empty, Monster, Empty, Empty, Empty, Monster, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Monster],
                    [Monster, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                ],
                top_nums: [4, 3, 4, 4, 3, 2, 6, 3],
                left_nums: [5, 5, 0, 7, 3, 3, 4, 2],
                seed: Some(16015493),
            },
            "ogre" => Puzzle {
                tiles: [
                    [Monster, Empty, Empty, Empty, Empty, Empty, Monster, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Treasure, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Monster, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Monster, Empty, Empty, Empty],
                    [Empty, Empty, Monster, Empty, Empty, Empty, Empty, Monster],
                ],
                top_nums: [0, 6, 2, 4, 3, 4, 3, 2],
                left_nums: [3, 1, 3, 5, 3, 1, 5, 3],
                seed: Some(35000071),
            },
            "demon" => Puzzle {
                tiles: [
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Monster, Empty, Empty, Empty, Empty, Empty, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Monster, Empty, Empty, Empty, Monster, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Treasure, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Monster, Empty, Empty, Monster],
                ],
                top_nums: [4, 2, 4, 2, 4, 6, 0, 6],
                left_nums: [4, 2, 5, 4, 6, 2, 3, 2],
                seed: Some(17827485),
            },

            "bear" => Puzzle {
                tiles: [
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Monster, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Monster, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Treasure, Empty, Monster, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Monster],
                ],
                top_nums: [7, 3, 3, 1, 6, 3, 5, 0],
                left_nums: [3, 4, 7, 1, 4, 2, 3, 4],
                seed: Some(56437193),
            },

            "cultist" => Puzzle {
                tiles: [
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Treasure, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Treasure],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Monster, Empty, Empty, Empty, Empty, Monster, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                ],
                top_nums: [3, 4, 1, 4, 6, 1, 3, 2],
                left_nums: [2, 2, 2, 5, 3, 4, 4, 2],
                seed: Some(31716032),
            },

            "lookseer" => Puzzle {
                tiles: [
                    [Empty, Treasure, Empty, Empty, Empty, Empty, Treasure, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Monster, Empty],
                ],
                top_nums: [5, 3, 3, 1, 5, 2, 3, 3],
                left_nums: [2, 2, 2, 5, 1, 5, 3, 5],
                seed: Some(63954165),
            },

            "golem" => Puzzle {
                tiles: [
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Treasure, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Treasure, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Monster, Empty, Monster, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Monster, Empty, Monster, Empty, Monster, Empty, Empty],
                ],
                top_nums: [3, 3, 3, 3, 6, 3, 2, 5],
                left_nums: [5, 2, 1, 4, 6, 4, 1, 5],
                seed: None,
            },

            "chest" => Puzzle {
                tiles: [
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Treasure, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Treasure, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Treasure, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                ],
                top_nums: [2, 2, 1, 6, 2, 1, 3, 4],
                left_nums: [2, 2, 2, 5, 5, 2, 1, 2],
                seed: None,
            },

            "squid" => Puzzle {
                tiles: [
                    [Empty, Treasure, Empty, Empty, Empty, Empty, Treasure, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Monster, Empty],
                ],
                top_nums: [5, 3, 3, 1, 5, 2, 3, 3],
                left_nums: [2, 2, 2, 5, 1, 5, 3, 5],
                seed: Some(63954165),
            },

            "slime" => Puzzle {
                tiles: [
                    [Empty, Empty, Monster, Empty, Empty, Empty, Monster, Empty],
                    [Empty, Empty, Empty, Empty, Monster, Empty, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Empty, Empty, Empty, Empty, Empty],
                    [Empty, Empty, Empty, Monster, Empty, Monster, Empty, Empty],
                    [Monster, Empty, Empty, Empty, Empty, Empty, Empty, Monster],
                    [Empty, Empty, Empty, Empty, Empty, Monster, Empty, Empty],
                ],
                top_nums: [4, 2, 4, 4, 3, 4, 3, 4],
                left_nums: [4, 4, 1, 5, 2, 4, 4, 4],
                seed: Some(6606032),
            },
            _ => Puzzle::default(),
        }
    }

    #[test_case("bear")]
    #[test_case("cultist")]
    #[test_case("demon")]
    #[test_case("goat")]
    #[test_case("goblin")]
    #[test_case("golem")]
    #[test_case("imp")]
    #[test_case("insectoid")]
    #[test_case("king")]
    #[test_case("kobold")]
    #[test_case("lich")]
    #[test_case("lookseer")]
    #[test_case("minotaur")]
    #[test_case("ogre")]
    #[test_case("skeleton")]
    #[test_case("slime")]
    #[test_case("squid")]
    /// Tests that images can be successfully parsed by comparing to reference results.
    fn parse(monster: &str) {
        // Open the image
        let path = std::path::Path::new("monster_refs").join(format!("{monster}.png"));
        let img = open(path).expect("{monster}.png not found").to_rgba8();

        // Parse it
        let p = Puzzle::from_image(img.view(0, 0, img.width(), img.height())).unwrap();

        // Print the incorrect result on failure for debugging
        assert!(p == get_reference_puzzle(monster), "{monster} => {p:?},")
    }

    #[test_case("bear")]
    #[test_case("chest")]
    #[test_case("cultist")]
    #[test_case("demon")]
    #[test_case("goat")]
    #[test_case("goblin")]
    #[test_case("golem")]
    #[test_case("imp")]
    #[test_case("insectoid")]
    #[test_case("king")]
    #[test_case("kobold")]
    #[test_case("lich")]
    #[test_case("lookseer")]
    #[test_case("minotaur")]
    #[test_case("ogre")]
    #[test_case("skeleton")]
    #[test_case("slime")]
    #[test_case("squid")]
    /// Test that a screenshot can be parsed, and the serialization/deserialization
    /// round trip doesn't clobber any data
    fn serialization(monster: &str) {
        // Open the image and do initial parsing.
        let path = Path::new("monster_refs").join(format!("{monster}.png"));
        let img = open(path).expect("Open {path}").to_rgba8();
        let original = Puzzle::from_image(img.view(0, 0, img.width(), img.height())).unwrap();

        // Create a buffer and serialize the puzzle into it.
        let write_buffer = Vec::with_capacity(1000);
        let mut cursor = Cursor::new(write_buffer);
        original.serialize(&mut cursor).unwrap();

        // Fetch the buffer back from the Cursor wrapper, and pass it to the deserializer
        let read_buffer = cursor.into_inner();
        let deserialized = Puzzle::deserialize(read_buffer.first_chunk::<26>().unwrap()).unwrap();

        assert!(
            deserialized == original,
            "Original:\n{original}\nDeserialized:\n{deserialized}"
        );
    }
}
