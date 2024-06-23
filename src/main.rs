#![feature(array_windows)]

use std::{thread, time::Duration};

use enigo::{Button, Coordinate, Direction, Enigo, InputResult, Mouse, Settings};
use xcap::{image::GenericImageView, Window};

const GAME_TITLE: &str = "Last Call BBS";
const GAME_CROP: (u32, u32) = (335, 459);

const CLICK_DELAY: u64 = 20;

type Image = xcap::image::ImageBuffer<xcap::image::Rgba<u8>, Vec<u8>>;

#[derive(Debug)]
enum InitError {
    BBSNotFound,
    XCapError(xcap::XCapError),
    EnigoError(enigo::NewConError),
    MouseCaptureError(enigo::InputError),
    ImageError(xcap::image::ImageError),
    SearchError(SearchError),
}

#[derive(Debug)]
enum SearchError {
    NotFound,
    MultipleResults(usize),
    OutOfBounds,
}

struct DungeonCrawler {
    enigo: Enigo,
    window: Window,
    offset: (u32, u32),
}

impl DungeonCrawler {
    fn new() -> Result<Self, InitError> {
        // Find 'Last Call BBS' window
        let windows = Window::all().unwrap();
        let window = windows
            .iter()
            .find(|win| win.title() == GAME_TITLE)
            .ok_or(InitError::BBSNotFound)?
            .clone();

        // Capture the screen
        let img = window
            .capture_image()
            .map_err(|e| InitError::XCapError(e))?;
        img.save("game.png").map_err(|e| InitError::ImageError(e))?;

        let dnd_offset = find_dnd_offset(&img).map_err(|e| InitError::SearchError(e))?;
        img.view(dnd_offset.0, dnd_offset.1, GAME_CROP.0, GAME_CROP.1)
            .to_image()
            .save("dnd.png")
            .map_err(|e| InitError::ImageError(e))?;

        let mut settings = Settings::default();
        settings.linux_delay = 0;
        // Locate DnD subwindow
        let mut dc = Self {
            enigo: Enigo::new(&settings).map_err(|e| InitError::EnigoError(e))?,
            offset: (
                window.x() as u32 + dnd_offset.0,
                window.y() as u32 + dnd_offset.1,
            ),
            window,
        };

        // Force a click to capture the mouse in the application
        dc.click(0, 0)
            .map_err(|e| InitError::MouseCaptureError(e))?;

        Ok(dc)
    }

    fn click(&mut self, x: u32, y: u32) -> InputResult<()> {
        let cx = (x + self.offset.0) as i32;
        let cy = (y + self.offset.1) as i32;
        self.enigo.move_mouse(cx, cy, Coordinate::Abs)?;
        thread::sleep(Duration::from_millis(CLICK_DELAY / 2));
        self.enigo.button(Button::Left, Direction::Click)?;
        thread::sleep(Duration::from_millis(CLICK_DELAY / 2));
        Ok(())
    }
}

fn find_dnd_offset(image: &Image) -> Result<(u32, u32), SearchError> {
    // Pattern of image bytes to uniquely locate the DnD subwindow. The chosen pattern
    // exists at 299,0 relative to the top left corner of the subwindow.
    const PATTERN_LEN: usize = 12;
    const PATTERN: [u8; PATTERN_LEN] = [69, 52, 56, 255, 237, 169, 135, 255, 181, 147, 131, 255];
    const PATTERN_OFFSET: (u32, u32) = (299, 0);

    // Iterate over sliding window of 12 bytes, considering only every 4th window (pixel alignment)
    let matches = image
        .array_windows::<PATTERN_LEN>()
        .step_by(4)
        .enumerate()
        .filter_map(|(i, &chunk)| {
            if chunk == PATTERN {
                // Given the window index, calculate x and y offsets. Wrapping
                // subtraction here simplifies the bounds check later
                Some((
                    (i as u32 % image.width()).wrapping_sub(PATTERN_OFFSET.0),
                    (i as u32 / image.width()).wrapping_sub(PATTERN_OFFSET.1),
                ))
            } else {
                None
            }
        })
        .collect::<Vec<(u32, u32)>>();

    use SearchError::*;
    match matches.len() {
        0 => Err(NotFound),
        1 => {
            let (x, y) = matches[0];
            if x > 625 || y > 80 {
                Err(OutOfBounds)
            } else {
                Ok(matches[0])
            }
        }
        n => Err(MultipleResults(n)),
    }
}

fn main() {
    let mut dc = match DungeonCrawler::new() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {:#?}", e);
            std::process::exit(1)
        }
    };

    println!("offset: {:?}", dc.offset);

    // repeat 5 times
    use rand::seq::SliceRandom;
    use rand::thread_rng;
    for _ in 0..8 {
        let mut coords = Vec::new();
        for i in 0..8 {
            for j in 0..8 {
                let cx = i * 33 + 66;
                let cy = j * 33 + 191;
                coords.push((cx, cy));
            }
        }
        coords.shuffle(&mut thread_rng());

        for (cx, cy) in coords {
            dc.click(cx, cy).unwrap();
        }

        thread::sleep(Duration::from_millis(250));
    }
}
