use std::u8;

use rand::Rng;

pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub fn get_rnd_color() -> Color {
    let mut rng = rand::thread_rng();
    let r = rng.gen_range(0..255);
    let g = rng.gen_range(0..255);
    let b = rng.gen_range(0..255);
    return Color { r, g, b };
}
