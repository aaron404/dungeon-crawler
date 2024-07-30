use std::{path::Path, thread, time::Duration};

use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};
use xcap::{
    image::{GenericImageView, RgbaImage},
    Window,
};

use anyhow::anyhow;
use anyhow::{Context, Result};

use crate::puzzle::Puzzle;

const GAME_TITLE: &str = "Last Call BBS";
const GAME_CROP: (u32, u32) = (335, 459);

const CLICK_DELAY: u64 = 20;
const SCREENSHOT_DELAY: u64 = 10;

const RANDOM_OFFSET: (u32, u32) = (285, 111);

pub struct DungeonCrawler {
    enigo: Enigo,
    dnd_img: RgbaImage,
    window: Window,
    dnd_offset: (u32, u32),
    win_offset: (u32, u32),
}

impl DungeonCrawler {
    pub fn new() -> Result<Self> {
        let plate = xcap::image::open("plate.png")?.to_rgba8();

        // Find 'Last Call BBS' window
        let windows = Window::all().unwrap();
        let window = windows
            .iter()
            .find(|win| win.title() == GAME_TITLE)
            .context("Failed to find Last Call BBS Window")?
            .clone();

        // Capture the screen
        let img = window.capture_image()?;
        #[cfg(debug_assertions)]
        img.save("game.png")?;

        let dnd_offset = find_dnd_offset(&img)?;

        let dnd_img = img
            .view(dnd_offset.0, dnd_offset.1, GAME_CROP.0, GAME_CROP.1)
            .to_image();

        #[cfg(debug_assertions)]
        dnd_img.save("dnd.png")?;

        let mut mask = plate.clone();
        mask.pixels_mut()
            .zip(dnd_img.pixels())
            .for_each(|(a, b)| match a.0 == b.0 {
                true => a.0 = [0, 0, 0, 255],
                false => a.0 = [255; 4],
            });

        #[cfg(debug_assertions)]
        mask.save("mask.png").unwrap();

        let mut dnd_img = dnd_img.clone();
        for (dst, src) in dnd_img.pixels_mut().zip(plate.pixels()) {
            dst.0[3] = match dst.0 == src.0 {
                true => 0,
                false => 255,
            }
        }

        #[cfg(debug_assertions)]
        dnd_img.save("dnd_img_alpha.png").unwrap();

        let mut settings = Settings::default();
        settings.linux_delay = 0;
        // Locate DnD subwindow
        let mut dc = Self {
            enigo: Enigo::new(&settings)?,
            dnd_img,
            dnd_offset,
            win_offset: (window.x() as u32, window.y() as u32),
            window,
        };

        // Force a click to capture the mouse in the application
        dc.click(0, 0)?;

        Ok(dc)
    }

    fn click(&mut self, x: u32, y: u32) -> Result<()> {
        let cx = (x + self.win_offset.0 + self.dnd_offset.0) as i32;
        let cy = (y + self.win_offset.1 + self.dnd_offset.1) as i32;
        self.enigo.move_mouse(cx, cy, Coordinate::Abs)?;
        thread::sleep(Duration::from_millis(CLICK_DELAY / 2));
        self.enigo.button(Button::Left, Direction::Click)?;
        thread::sleep(Duration::from_millis(CLICK_DELAY / 2));
        Ok(())
    }

    pub fn random_board(&mut self) {
        self.click(RANDOM_OFFSET.0, RANDOM_OFFSET.1).unwrap();
        thread::sleep(Duration::from_millis(SCREENSHOT_DELAY));
        self.dnd_img = self
            .window
            .capture_image()
            .expect("failed to capture image")
            .view(
                self.dnd_offset.0,
                self.dnd_offset.1,
                GAME_CROP.0,
                GAME_CROP.1,
            )
            .to_image();
    }

    pub fn parse(&mut self) -> Result<Puzzle> {
        let img = self.window.capture_image()?;
        let img = img.view(
            self.dnd_offset.0,
            self.dnd_offset.1,
            GAME_CROP.0,
            GAME_CROP.1,
        );
        let puzzle = Puzzle::from_image(img)?;

        #[cfg(debug_assertions)]
        puzzle.draw_parsing_overlay(img);

        Ok(puzzle)
    }

    pub fn reset_solution(&mut self) -> Result<()> {
        self.click(74, 33)?;
        thread::sleep(Duration::from_millis(250));
        self.click(74, 93)
    }

    pub fn place_wall(&mut self, x: u8, y: u8) -> Result<()> {
        self.click(x as u32 * 33 + 66, y as u32 * 33 + 191)
    }

    pub fn enter_solution(&mut self, solution: u64) -> Result<()> {
        println!("{solution}");
        for i in 0..64 {
            let y = i / 8;
            let x = i % 8;
            if solution & (1 << (63 - i)) != 0 {
                self.place_wall(x, y)?;
            }
        }
        self.click(0, 0)?;
        thread::sleep(Duration::from_millis(10));
        Ok(())
    }

    pub fn save_board_image(&self, crop: Option<(u32, u32, u32, u32)>, path: &Path) -> Result<()> {
        let img = self.window.capture_image()?;
        let (x, y, w, h) = match crop {
            Some((x, y, w, h)) => (x, y, w, h),
            None => (self.dnd_offset.0 + 10, self.dnd_offset.1 + 135, 310, 310),
        };
        img.view(x, y, w, h).to_image().save(path)?;

        Ok(())
    }
}

fn find_dnd_offset(image: &RgbaImage) -> Result<(u32, u32)> {
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

    match matches.len() {
        0 => Err(anyhow!("Not found")),
        1 => {
            let (x, y) = matches[0];
            if x > 625 || y > 80 {
                Err(anyhow!("Out of bounds"))
            } else {
                Ok(matches[0])
            }
        }
        n => Err(anyhow!("Multiple matches ({n})")),
    }
}
