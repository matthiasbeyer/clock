pub fn rainbow_color_iterator() -> impl Iterator<Item = embedded_graphics::pixelcolor::Rgb888> {
    fn hsv_to_rgb(hue: f32) -> (u8, u8, u8) {
        let h = hue % 1.0;
        let c = 1.0;
        let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
        let m = 0.0;

        let (r, g, b) = match (h * 6.0) as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            5 => (c, 0.0, x),
            _ => (0.0, 0.0, 0.0),
        };

        (
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
        )
    }

    std::iter::successors(Some(0.0), |&t| Some((t + 0.01) % 1.0))
        .map(hsv_to_rgb)
        .map(|(r, g, b)| embedded_graphics::pixelcolor::Rgb888::new(r, g, b))
}
