pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let c = v * s;
    let h_prime = (h * 6.0) % 6.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h_prime as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [r + m, g + m, b + m]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hsv_to_rgb_red() {
        let rgb = hsv_to_rgb(0.0, 1.0, 1.0);
        assert!((rgb[0] - 1.0).abs() < 0.01);
        assert!(rgb[1].abs() < 0.01);
        assert!(rgb[2].abs() < 0.01);
    }

    #[test]
    fn test_hsv_to_rgb_white() {
        let rgb = hsv_to_rgb(0.0, 0.0, 1.0);
        assert!((rgb[0] - 1.0).abs() < 0.01);
        assert!((rgb[1] - 1.0).abs() < 0.01);
        assert!((rgb[2] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_hsv_to_rgb_black() {
        let rgb = hsv_to_rgb(0.0, 1.0, 0.0);
        assert!(rgb[0].abs() < 0.01);
        assert!(rgb[1].abs() < 0.01);
        assert!(rgb[2].abs() < 0.01);
    }
}
