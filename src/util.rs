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
    Color { r, g, b }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_randomness() {
        let color1 = get_rnd_color();
        let color2 = get_rnd_color();
        assert!(color1.r != color2.r || color1.g != color2.g || color1.b != color2.b);
    }
}
