use std::time::Instant;

use crate::puzzle::{Puzzle, Tile};

pub trait Solver {
    fn solve(&self, puzzle: &Puzzle) -> Vec<u64>;
}
pub struct BackTracker;

impl Solver for BackTracker {
    fn solve(&self, puzzle: &Puzzle) -> Vec<u64> {
        let mut solutions = Vec::new();

        let now = Instant::now();
        BackTracker::solve_recursive(puzzle, puzzle.top_nums, 0, &mut solutions);

        println!(
            "{} solutions found in {:.2}s",
            solutions.len(),
            now.elapsed().as_secs_f32()
        );

        return solutions;
        // match solutions.len() {
        //     0 => Err(SolverError::None),
        //     1 => Ok(solutions),
        //     _ => Err(SolverError::Multiple),
        // }
    }
}

impl BackTracker {
    pub fn solve_recursive(
        puzzle: &Puzzle,
        col_counts: [u8; 8],
        depth: usize,
        solutions: &mut Vec<u64>,
    ) {
        let row_wall_count = puzzle.left_nums[depth];

        let empty_cells_mask = puzzle.tiles[depth]
            .into_iter()
            .map(|tile| if tile == Tile::Empty { 1u8 } else { 0 })
            .fold(0, |acc, x| (acc << 1) + x);
        let needs_walls_mask = col_counts
            .into_iter()
            .map(|i| if i > 0 { 1u8 } else { 0 })
            .fold(0, |acc, x| (acc << 1) + x);

        let mask = empty_cells_mask & needs_walls_mask;

        if mask.count_ones() < row_wall_count.into() {
            return;
        } else if mask.count_ones() > row_wall_count.into() && depth == 7 {
            return;
        }

        // let mut solution_found = false;
        // let mut solution = 0u64;

        let mut last_mask = u8::MAX;
        for candidate_mask in 0..255 {
            let walls_to_place_mask = candidate_mask & mask;
            if walls_to_place_mask.count_ones() as u8 != row_wall_count {
                continue;
            }

            if walls_to_place_mask == last_mask {
                continue;
            } else {
                last_mask = walls_to_place_mask;
            }

            // for _ in 0..depth {
            //     print!(" ");
            // }
            // println!("{walls_to_place_mask:08b}");

            let mut puzzle = puzzle.clone();
            let mut counts = col_counts.clone();
            for i in 0..8 {
                if (walls_to_place_mask & (1 << (7 - i))) >= 1 {
                    puzzle.tiles[depth][i] = Tile::Wall;
                    counts[i] -= 1;
                }
            }

            if depth == 7 {
                if is_valid_solution(&puzzle, false) {
                    let sol = walls_to_solution(&puzzle);
                    if solutions.contains(&sol) {
                        // println!("exists");
                    } else {
                        is_valid_solution(&puzzle, false);
                        solutions.push(sol);
                    }
                }
            } else {
                BackTracker::solve_recursive(&puzzle, counts, depth + 1, solutions);
            }
        }
    }
}

fn walls_to_solution(puzzle: &Puzzle) -> u64 {
    puzzle.tiles.as_flattened().iter().fold(0u64, |acc, &tile| {
        (acc << 1) + if tile == Tile::Wall { 1 } else { 0 }
    })
}

fn print_solution(sol: u64) {
    println!("{sol:064b}");
    for i in (0..8).rev() {
        let row = ((sol >> (i * 8)) & 0xff) as u8;
        println!("  {:08b}", row);
    }
}

fn is_valid_solution(puzzle: &Puzzle, debug: bool) -> bool {
    // look for contradictions
    let monster_mask = puzzle.tiles.as_flattened().iter().fold(0u64, |acc, &tile| {
        (acc << 1) + if tile == Tile::Monster { 1 } else { 0 }
    });

    // let wall_mask = puzzle.tiles.as_flattened().iter().fold(0u64, |acc, &tile| {
    //     (acc << 1) + if tile == Tile::Wall { 1 } else { 0 }
    // });

    let treasure_mask = puzzle.tiles.as_flattened().iter().fold(0u64, |acc, &tile| {
        (acc << 1) + if tile == Tile::Treasure { 1 } else { 0 }
    });

    let empty_mask = puzzle.tiles.as_flattened().iter().fold(0u64, |acc, &tile| {
        (acc << 1) + if tile == Tile::Empty { 1 } else { 0 }
    });

    // Monsters should have exactly 1 empty tile next to them
    let has_empty_above = (empty_mask >> 8) & monster_mask;
    let has_empty_below = (empty_mask << 8) & monster_mask;
    let has_empty_right = (empty_mask << 1) & monster_mask & 0xfefefefefefefefe;
    let has_empty_left = (empty_mask >> 1) & monster_mask & 0x7f7f7f7f7f7f7f7f;

    if has_empty_above & has_empty_right > 0
        || has_empty_above & has_empty_left > 0
        || has_empty_above & has_empty_below > 0
        || has_empty_left & has_empty_right > 0
        || has_empty_left & has_empty_below > 0
        || has_empty_right & has_empty_below > 0
    {
        return false;
    }

    // correction for monster with 4 walls
    if (!has_empty_above & !has_empty_left & !has_empty_right & !has_empty_below & monster_mask) > 0
    {
        return false;
    }

    // All corridors should be connected
    // Find an empty cell and do a floodfill, counting the size of the region
    let tz = empty_mask.trailing_zeros();
    let mut flood_mask = 1u64 << tz;

    let e_mask = empty_mask | treasure_mask;
    loop {
        let mut flood_mask2 = flood_mask;
        flood_mask2 |= ((flood_mask << 1) & 0xfefefefefefefefe) & e_mask;
        flood_mask2 |= ((flood_mask2 >> 1) & 0x7f7f7f7f7f7f7f7f) & e_mask;
        flood_mask2 |= (flood_mask2 << 8) & e_mask;
        flood_mask2 |= (flood_mask2 >> 8) & e_mask;
        // println!("flood_mask2:");
        // print_solution(flood_mask2);
        if flood_mask2 == flood_mask {
            break;
        }
        flood_mask = flood_mask2;
    }

    if flood_mask.count_ones() != empty_mask.count_ones() + treasure_mask.count_ones() {
        return false;
    }

    // Find location of treasure rooms
    let e_mask = empty_mask | treasure_mask;
    let mut room_mask = e_mask;
    room_mask = room_mask & ((e_mask << 1) & 0xfefefefefefefefe);
    room_mask = room_mask & ((e_mask << 2) & 0xfcfcfcfcfcfcfcfc);
    if debug {
        println!("empty mask");
        print_solution(e_mask);
        println!("treasure room mask");
        print_solution(room_mask);
    }

    room_mask = room_mask & (room_mask << 8) & (room_mask << 16);
    if debug {
        println!("treasure room mask2");
        print_solution(room_mask);
    }

    if room_mask.count_ones() != treasure_mask.count_ones() {
        return false;
    }

    // dead ends must have monster
    // 98679005

    true
}
