use std::{fs, path::Path};
use xcap::image::{self, DynamicImage, GenericImageView};

// Compute the squared error between two 4-byte pixels. Ignore
// pixels that are not fully opaque.
#[allow(dead_code)]
fn compare_pixels(haystack: [u8; 4], needle: [u8; 4]) -> u64 {
    if needle[3] < 255 {
        0
    } else {
        let mut sum = 0u64;
        for i in 0..3 {
            let e = haystack[i].abs_diff(needle[i]) as u64;
            sum += e * e;
        }
        sum
    }
}

// Compute sum of square error between two images at every offset, return position of lowest score
#[allow(dead_code)]
fn compare_images(haystack: &DynamicImage, needle: &DynamicImage) -> (u64, (u32, u32)) {
    let nw = needle.width();
    let nh = needle.height();

    // Keep track of best score and the position it was achieved
    let mut best_score: u64 = u64::MAX;
    let mut best_pos: (u32, u32) = (0, 0);
    // Slide the needle image over the haystack
    for x in 0..haystack.width() - nw {
        for y in 0..haystack.height() - nh {
            // Create a sub-view of the haystack
            let view = haystack.view(x, y, nw, nh);

            // Copmute sum of squared errors
            let sse = view
                .pixels()
                .map(|(_, _, p)| p)
                .zip(needle.pixels().map(|(_, _, p)| p))
                .fold(0, |acc, (p1, p2)| acc + compare_pixels(p1.0, p2.0));

            // Check if we found a better match
            if sse < best_score {
                // Perfect match, return early
                if sse == 0 {
                    return (sse, (x, y));
                }
                best_score = sse;
                best_pos = (x, y);
            }
        }
    }
    return (best_score, best_pos);
}

const TILES_OFFSET: (u32, u32) = (49, 175);
const TILE_STRIDE: u32 = 33;
#[allow(dead_code)]
fn correct_offset(size: (u32, u32), offset: (u32, u32)) -> (i32, i32) {
    let center_x = size.0 / 2;
    let center_y = size.1 / 2;

    // Normalize coordinates so first tile is at (0, 0), and use the
    // center of the sprite rather than its corner to keep it positive.
    let offset_x = offset.0 + center_x - TILES_OFFSET.0;
    let offset_y = offset.1 + center_y - TILES_OFFSET.1;

    // Modulo shifts the value to something within the top-left tile.
    let offset_x = offset_x % TILE_STRIDE;
    let offset_y = offset_y % TILE_STRIDE;

    // Subtract the centerpoint to get the corrected offset
    (
        center_x as i32 - offset_x as i32,
        center_y as i32 - offset_y as i32,
    )
}

#[allow(dead_code)]
pub fn find_monster_offsets() {
    // Iterate over each monster image folder
    for dir in fs::read_dir("tokyo").unwrap() {
        let dir = dir.unwrap();
        let monster = dir.file_name();

        print!("{},", monster.to_string_lossy());
        // print!("{: <10}", monster.to_string_lossy());

        // Open reference image
        let board_path = Path::new("monster_refs").join(Path::new(&monster).with_extension("png"));
        if let Ok(board) = image::open(board_path) {
            let mut best_score = u64::MAX;
            let mut best_pos = (0, 0);
            // let mut best_frame = OsString::new();

            let mut size = (0, 0);
            // Iterate over each frame to find best match
            for frame in fs::read_dir(dir.path()).unwrap() {
                let frame = frame.unwrap();
                let monster_img = image::open(frame.path()).unwrap();
                size = (monster_img.width(), monster_img.height());
                let (score, pos) = compare_images(&board, &monster_img);
                if score < best_score {
                    best_score = score;
                    best_pos = pos;
                    // best_frame = frame.file_name();
                }
            }
            let offset = correct_offset(size, best_pos);
            println!("{:+}{:+}", offset.0, offset.1);
            // println!(
            //     "{: <6} {offset: >+3?} {best_score}",
            //     best_frame.to_string_lossy()
            // );
        } else {
            println!("--  skipped  --");
            continue;
        }
    }
}
