use xcap::image::RgbaImage;

pub fn draw_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, col: [u8; 4]) {
    for j in y..y + h {
        for i in x..x + w {
            let p = img.get_pixel_mut(i, j);
            let sa = col[3] as u32;
            let da = p.0[3] as u32;
            if da == 0 {
                p.0 = col;
            } else {
                for i in 0..3 {
                    p.0[i] = ((col[i] as u32 * sa + p.0[i] as u32 * da) / (sa + da)) as u8;
                }
            }
        }
    }
}
