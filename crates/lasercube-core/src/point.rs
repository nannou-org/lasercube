//! Point data representation for laser rendering.

/// A single point to be rendered by the laser.
///
/// Coordinates are in the range 0-0xFFF, with 0x800 being the center.
/// Color values are in the range 0-0xFFF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    /// Each coordinate (0x000-0xFFF, 0x800 is center)
    pub pos: Position,
    /// Red, green, blue channel intensities (0x000-0xFFF)
    pub rgb: Rgb,
}

/// Each coordinate (0x000-0xFFF, 0x800 is center)
pub type Position = [u16; 2];

/// Red, green, blue channel intensities (0x000-0xFFF)
pub type Rgb = [u16; 3];

impl Point {
    /// Center coordinate value.
    pub const CENTER_COORD: u16 = 0x800;
    /// Center position value.
    pub const CENTER_POS: Position = [Self::CENTER_COORD; 2];
    /// Maximum coordinate value (12-bit).
    pub const MAX_COORD: u16 = 0xFFF;
    /// Maximum color value (12-bit).
    pub const MAX_COLOR: u16 = 0xFFF;
    /// A blank RGB color.
    pub const BLANK: Rgb = [0; 3];
    /// A centered, blank point.
    pub const CENTER_BLANK: Self = Self::new(Self::CENTER_POS, Self::BLANK);
    /// Size of a point in bytes when serialized. 5 * u16
    pub const SIZE: usize = 10;

    /// Create a new point with the given coordinates and color.
    pub const fn new(pos: Position, rgb: Rgb) -> Self {
        Self { pos, rgb }
    }

    /// Create a point from normalized coordinates and colors.
    ///
    /// Coordinates should be in the range [-1.0, 1.0], with (0.0, 0.0) being the center.
    /// Colors should be in the range [0.0, 1.0].
    pub fn from_normalized([x, y]: [f32; 2], [r, g, b]: [f32; 3]) -> Self {
        let x = coord_from_normalized(x);
        let y = coord_from_normalized(y);
        let r = color_from_normalized(r);
        let g = color_from_normalized(g);
        let b = color_from_normalized(b);
        Self::new([x, y], [r, g, b])
    }

    /// Convert to normalized coordinates and colors.
    ///
    /// Returns coordinates in the range [-1.0, 1.0], with (0.0, 0.0) being the center.
    /// Returns colors in the range [0.0, 1.0].
    pub fn to_normalized(&self) -> ([f32; 2], [f32; 3]) {
        let x_norm = normalized_from_coord(self.pos[0]);
        let y_norm = normalized_from_coord(self.pos[1]);
        let r_norm = normalized_from_color(self.rgb[0]);
        let g_norm = normalized_from_color(self.rgb[1]);
        let b_norm = normalized_from_color(self.rgb[2]);
        ([x_norm, y_norm], [r_norm, g_norm, b_norm])
    }
}

impl From<Point> for [u8; Point::SIZE] {
    fn from(p: Point) -> Self {
        let ([x, y], [r, g, b]) = (p.pos, p.rgb);
        let [x0, x1] = x.to_le_bytes();
        let [y0, y1] = y.to_le_bytes();
        let [r0, r1] = r.to_le_bytes();
        let [g0, g1] = g.to_le_bytes();
        let [b0, b1] = b.to_le_bytes();
        [x0, x1, y0, y1, r0, r1, g0, g1, b0, b1]
    }
}

impl From<[u8; Point::SIZE]> for Point {
    fn from([x0, x1, y0, y1, r0, r1, g0, g1, b0, b1]: [u8; Point::SIZE]) -> Self {
        let x = u16::from_le_bytes([x0, x1]);
        let y = u16::from_le_bytes([y0, y1]);
        let r = u16::from_le_bytes([r0, r1]);
        let g = u16::from_le_bytes([g0, g1]);
        let b = u16::from_le_bytes([b0, b1]);
        Point::new([x, y], [r, g, b])
    }
}

/// Produce a `Point`-compatible coordinate from a normalized coordinate.
pub fn coord_from_normalized(coord_norm: f32) -> u16 {
    let normalized = coord_norm.max(-1.0).min(1.0);
    let scaled = ((normalized + 1.0) / 2.0) * Point::MAX_COORD as f32;
    scaled as u16
}

/// Produce a `Point`-compatible color value from a normalized color value.
pub fn color_from_normalized(color_norm: f32) -> u16 {
    let normalized = color_norm.max(0.0).min(1.0);
    let scaled = normalized * Point::MAX_COLOR as f32;
    scaled as u16
}

/// Produce a normalized coordinate from a `Point`-compatible coordinate.
pub fn normalized_from_coord(coord: u16) -> f32 {
    (coord as f32 / Point::MAX_COORD as f32) * 2.0 - 1.0
}

/// Produce a normalized color value from a `Point`-compatible color value.
pub fn normalized_from_color(color: u16) -> f32 {
    color as f32 / Point::MAX_COLOR as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_size() {
        assert_eq!(std::mem::size_of::<Point>(), Point::SIZE);
    }

    #[test]
    fn test_point_new() {
        let p = Point::new([0x800, 0x800], [0x800, 0x400, 0]);
        assert_eq!(p.pos[0], 0x800);
        assert_eq!(p.pos[1], 0x800);
        assert_eq!(p.rgb[0], 0x800);
        assert_eq!(p.rgb[1], 0x400);
        assert_eq!(p.rgb[2], 0);
    }

    #[test]
    fn test_normalization_functions() {
        // Test coordinate normalization
        let coord = 0x800; // Center
        let norm = normalized_from_coord(coord);
        assert!((norm - 0.0).abs() < 0.01);
        let back_to_coord = coord_from_normalized(norm);
        assert_eq!(back_to_coord, coord);

        // Test full range
        let coord_min = 0;
        let norm_min = normalized_from_coord(coord_min);
        assert!((norm_min - (-1.0)).abs() < 0.01);

        let coord_max = Point::MAX_COORD;
        let norm_max = normalized_from_coord(coord_max);
        assert!((norm_max - 1.0).abs() < 0.01);

        // Test color normalization
        let color = Point::MAX_COLOR / 2;
        let norm = normalized_from_color(color);
        assert!((norm - 0.5).abs() < 0.01);
        let back_to_color = color_from_normalized(norm);
        assert_eq!(back_to_color, color);

        // Test color range
        let color_min = 0;
        let norm_min = normalized_from_color(color_min);
        assert!((norm_min - 0.0).abs() < 0.01);

        let color_max = Point::MAX_COLOR;
        let norm_max = normalized_from_color(color_max);
        assert!((norm_max - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_round_trip() {
        // Test that normalizing and then denormalizing gives the same value
        for coord in [0, 0x400, 0x800, 0xC00, 0xFFF] {
            let norm = normalized_from_coord(coord);
            let back = coord_from_normalized(norm);
            assert_eq!(back, coord, "Coordinate round-trip failed for {}", coord);
        }

        for color in [0, 0x400, 0x800, 0xC00, 0xFFF] {
            let norm = normalized_from_color(color);
            let back = color_from_normalized(norm);
            assert_eq!(back, color, "Color round-trip failed for {}", color);
        }

        // Test complete point round-trip
        let original = Point::new([0x400, 0xC00], [0x800, 0, 0xFFF]);
        let (pos_norm, rgb_norm) = original.to_normalized();
        let restored = Point::from_normalized(pos_norm, rgb_norm);

        // Due to floating point precision, we might lose 1-2 bits, so check within a small tolerance
        assert!((restored.pos[0] as i32 - original.pos[0] as i32).abs() <= 1);
        assert!((restored.pos[1] as i32 - original.pos[1] as i32).abs() <= 1);
        assert!((restored.rgb[0] as i32 - original.rgb[0] as i32).abs() <= 1);
        assert!((restored.rgb[1] as i32 - original.rgb[1] as i32).abs() <= 1);
        assert!((restored.rgb[2] as i32 - original.rgb[2] as i32).abs() <= 1);
    }

    #[test]
    fn test_bytes() {
        let point = Point::new([0x1234, 0x5678], [0x9ABC, 0xDEF0, 0x1234]);

        // Convert to bytes
        let bytes: [u8; Point::SIZE] = point.into();

        // Check little-endian byte representation
        assert_eq!(bytes[0], 0x34); // low byte of x
        assert_eq!(bytes[1], 0x12); // high byte of x
        assert_eq!(bytes[2], 0x78); // low byte of y
        assert_eq!(bytes[3], 0x56); // high byte of y
        assert_eq!(bytes[4], 0xBC); // low byte of r
        assert_eq!(bytes[5], 0x9A); // high byte of r
        assert_eq!(bytes[6], 0xF0); // low byte of g
        assert_eq!(bytes[7], 0xDE); // high byte of g
        assert_eq!(bytes[8], 0x34); // low byte of b
        assert_eq!(bytes[9], 0x12); // high byte of b

        // Convert back to Point
        let restored = Point::from(bytes);

        // Check that we got the original point back
        assert_eq!(restored.pos[0], point.pos[0]);
        assert_eq!(restored.pos[1], point.pos[1]);
        assert_eq!(restored.rgb[0], point.rgb[0]);
        assert_eq!(restored.rgb[1], point.rgb[1]);
        assert_eq!(restored.rgb[2], point.rgb[2]);
    }
}
