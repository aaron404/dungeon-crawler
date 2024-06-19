use std::{thread, time::Duration};

use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};
use xcap::Window;

const GAME_TITLE: &str = "Last Call BBS";

fn main() {
    // Create an iterator over all the open windows
    let windows = Window::all().unwrap();

    // Search for our specific window
    let window = windows
        .iter()
        .find(|win| win.title() == GAME_TITLE)
        .unwrap();

    // Initialize enigo input simulator
    let mut enigo = Enigo::new(&Settings::default()).unwrap();

    // Move the mouse
    enigo
        .move_mouse(
            window.x() + window.width() as i32 / 2,
            window.y() + window.height() as i32 / 2,
            Coordinate::Abs,
        )
        .unwrap();

    // Click
    enigo.button(Button::Left, Direction::Click).unwrap();

    thread::sleep(Duration::from_millis(100));

    // Take a screenshot
    let img = window.capture_image().unwrap();
    img.save("game.png").unwrap();
}
