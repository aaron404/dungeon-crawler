#![feature(array_windows)]
#![feature(array_chunks)]
#![feature(iter_array_chunks)]
#![feature(path_file_prefix)]

use std::{
    collections::HashSet,
    env,
    fs::{self, OpenOptions},
    io::{Cursor, Write},
    path::Path,
    process::exit,
    thread,
    time::{Duration, Instant},
};

mod dungeon_crawler;
mod monster_search;
mod puzzle;
mod scripts;
mod solve;
mod tex;
mod util;

use anyhow::Result;
use puzzle::Puzzle;
use solve::Solver;

const DB_PATH: &str = "data";
const DB_FILE: &str = "puzzles.db";
const PUZZLES_PER_BATCH: usize = 4700;
#[allow(dead_code)]
fn collect_puzzles() -> Result<()> {
    // Create db file if it doesn't exist
    let db_path = Path::new(DB_PATH).join(DB_FILE);
    if !db_path.exists() {
        fs::create_dir_all(DB_PATH)?;
    }

    // Instantiate dungeon crawler and open db file in append mode
    let mut dc = dungeon_crawler::DungeonCrawler::new()?;
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(db_path)
        .expect("Database file should have been created");

    // Create buffer to serialize puzzles into
    let buffer = Vec::new();
    let mut cursor = Cursor::new(buffer);

    let t0 = Instant::now();

    for _ in 0..PUZZLES_PER_BATCH {
        dc.random_board();
        let puzzle = dc.parse()?;
        puzzle.serialize(&mut cursor)?;
    }

    // Append buffer to file
    file.write_all(&cursor.into_inner())?;

    let elapsed = t0.elapsed();
    println!(
        "{PUZZLES_PER_BATCH} parsed in {:.02}s. ({:.02}/s)",
        elapsed.as_secs_f32(),
        PUZZLES_PER_BATCH as f32 / elapsed.as_secs_f32()
    );

    Ok(())
}

#[allow(dead_code)]
fn read_puzzles() -> Result<Vec<Puzzle>> {
    let db_path = Path::new(DB_PATH).join(DB_FILE);
    let buffer = fs::read(db_path)?;

    let puzzles = buffer
        .array_chunks::<26>()
        .map(|chunk| puzzle::Puzzle::deserialize(chunk))
        .collect::<Result<Vec<puzzle::Puzzle>>>()?;

    Ok(puzzles)
}

#[allow(dead_code)]
fn print_db_info() -> Result<()> {
    let puzzles = read_puzzles()?;

    let seeds: HashSet<u32> = HashSet::from_iter(
        puzzles
            .iter()
            .map(|puzzle| puzzle.seed.unwrap_or(10000000u32)),
    );

    println!("Puzzle database");
    println!("  count: {}", puzzles.len());
    println!("  unique: {}", seeds.len());

    Ok(())
}

#[allow(dead_code)]
fn parse() -> Result<()> {
    let mut dc = dungeon_crawler::DungeonCrawler::new()?;
    dc.parse()?;
    Ok(())
}

fn solve() -> Result<()> {
    let mut dc = dungeon_crawler::DungeonCrawler::new()?;
    let bt = solve::BackTracker {};

    loop {
        let puzzle = dc.parse()?;
        println!("{puzzle}");
        println!("seed: {:?}", puzzle.seed);
        let solutions = bt.solve(&puzzle);

        // output dir
        // let path = Path::new("script_output").join("solve_bt");
        // let mut count = 0;
        // for solution in solutions.into_iter().step_by(5000) {
        // for solution in solutions.into_iter() {
        // dc.enter_solution(solution)?;
        // dc.save_board_image(None, &path.join(format!("{count:>04}.png")))?;
        // dc.reset_solution()?;
        // count += 1;
        // }

        match solutions.len() {
            0 => println!("  no solution"),
            1 => {
                dc.enter_solution(*solutions.last().unwrap()).unwrap();
            }
            _ => {
                println!("  multiple solutions");
                for sol in solutions {
                    println!("    {:064b}", sol);
                }
            }
        }

        thread::sleep(Duration::from_millis(2500));
        dc.random_board()
    }
}

fn main() -> Result<()> {
    if env::args().count() > 1 {
        tex::decode_all_textures();
        exit(0);
    }

    // parse()?;
    // collect_puzzles()?;
    // print_db_info()?;
    // do_stuff();

    solve()?;

    Ok(())
}

#[allow(dead_code)]
fn do_stuff() {
    // scripts::get_large_digit_discriminant();
    // scripts::tile_bg_colors();
    // scripts::find_monster_sample_offset();
    // scripts::print_background_pixels();
}
