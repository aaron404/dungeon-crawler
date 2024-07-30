// #![allow(dead_code)]

use byteorder::{LittleEndian, ReadBytesExt};
use std::{
    fs::{self, File},
    io::{Cursor, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

const DATA_DIR_ROOT: &str = "/home/aaron/.local/share/Steam/steamapps/common/Last Call BBS/Content";
const DATA_DIR_PREFIX: &str = r"/home/aaron/.local/share/Steam/steamapps/common/Last Call BBS/";

const TEX_SUFFIX: &str = ".tex";
const ARRAY_TEX_SUFFIX: &str = ".array.tex";

fn parse_array_tex(src_path: &Path, dest_path: PathBuf) {
    let path = PathBuf::from(src_path);

    let mut buffer = Vec::new();
    File::open(path).unwrap().read_to_end(&mut buffer).unwrap();
    let mut rdr = Cursor::new(buffer);
    let mut compressed = Vec::new();

    // skip magic number
    rdr.seek(SeekFrom::Current(4)).unwrap();
    rdr.seek(SeekFrom::Current(4)).unwrap();

    let mut i = 0;
    while let Ok(width) = rdr.read_u32::<LittleEndian>() {
        let height = rdr.read_u32::<LittleEndian>().unwrap();
        rdr.seek(SeekFrom::Current(56)).unwrap();

        let payload_size = rdr.read_u32::<LittleEndian>().unwrap();
        compressed.resize(payload_size as usize, 0);
        rdr.read_exact(&mut compressed).unwrap();

        let texture = lz4_flex::decompress(&compressed, 100000000).unwrap();
        let img = image::RgbaImage::from_raw(width, height, texture).unwrap();
        let img = image::imageops::flip_vertical(&img);

        let fname = dest_path.join(format!("{i:02}.png")); // format!("{}/{i:02}.png", dest_path.to_str().unwrap());
        img.save(fname).unwrap();
        i += 1;
    }
}

fn parse_tex(src_path: &Path, dest_path: PathBuf) {
    let path = PathBuf::from(src_path);

    // Storage for the file and compressed image data
    let mut buffer = Vec::new();
    let mut compressed = Vec::new();

    File::open(path).unwrap().read_to_end(&mut buffer).unwrap();

    // Cursor is used for file navigation, skip first 8 bytes
    let mut rdr = Cursor::new(buffer);
    rdr.seek(SeekFrom::Current(8)).unwrap();

    // Read width and height, then skip more metadata
    let width = rdr.read_u32::<LittleEndian>().unwrap();
    let height = rdr.read_u32::<LittleEndian>().unwrap();
    let format = rdr.read_u32::<LittleEndian>().unwrap();
    rdr.seek(SeekFrom::Current(52)).unwrap();

    // Read payload size, then read the payload
    let payload_size = rdr.read_u32::<LittleEndian>().unwrap();
    compressed.resize(payload_size as usize, 0);
    rdr.read_exact(&mut compressed).unwrap();

    // Decompress the texture, flip it vertically, and save it
    println!("  width: {width}, height: {height}, payload size: {payload_size}");
    let texture = lz4_flex::decompress(&compressed, 100000000).unwrap();
    println!("  uncompressed: {}", texture.len());
    println!("  format: {}", format);

    if format == 1 {
        let img = image::GrayImage::from_raw(width, height, texture).unwrap();
        let img = image::imageops::flip_vertical(&img);
        img.save(dest_path).unwrap();
    } else if format == 2 {
        let img = image::RgbaImage::from_raw(width, height, texture).unwrap();
        let img = image::imageops::flip_vertical(&img);
        img.save(dest_path).unwrap();
    } else {
        panic!();
    }
}

use walkdir::WalkDir;
use xcap::image;

pub fn decode_all_textures() {
    let decode_dir = Path::new("tokyo");
    if !Path::exists(decode_dir) {
        fs::create_dir(decode_dir).unwrap();
    }

    for entry in WalkDir::new(DATA_DIR_ROOT).into_iter().filter_map(|e| e.ok()) {
        let short_path = entry.path().strip_prefix(DATA_DIR_PREFIX).unwrap();
        if entry.file_type().is_dir() {
            match fs::create_dir(short_path) {
                Ok(()) => println!("Created folder: {}", short_path.display()),
                Err(_) => println!("Folder {} already exists", short_path.display()),
            }
        } else if entry.file_type().is_file() {
            println!("  {}", entry.path().to_string_lossy());
            if entry.path().to_str().unwrap().ends_with(ARRAY_TEX_SUFFIX) {
                let dname = entry.path().file_prefix().unwrap();
                fs::create_dir(short_path.parent().unwrap().join(dname)).ok();
                parse_array_tex(entry.path(), short_path.parent().unwrap().join(dname));
            } else if entry.path().to_str().unwrap().ends_with(TEX_SUFFIX) {
                parse_tex(entry.path(), short_path.with_extension("png"))
            }
        }
    }
}
