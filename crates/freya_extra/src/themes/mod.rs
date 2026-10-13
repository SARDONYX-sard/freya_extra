pub mod dark_glass;
pub mod editor;
pub mod liquid_glass;

use freya::prelude::Color;

pub const fn hex(value: &str) -> Color {
    const fn parse_byte(bytes: &[u8], index: usize) -> u8 {
        (parse_nibble(bytes[index]) << 4) | parse_nibble(bytes[index + 1])
    }

    const fn parse_nibble(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => panic!("Invalid hex digit"),
        }
    }

    const fn expand_nibble(value: u8) -> u8 {
        (value << 4) | value
    }

    let bytes = value.as_bytes();
    let offset = if !bytes.is_empty() && bytes[0] == b'#' { 1 } else { 0 };

    let len = bytes.len() - offset;

    let (r, g, b, a) = match len {
        // #RGB
        3 => (
            expand_nibble(parse_nibble(bytes[offset])),
            expand_nibble(parse_nibble(bytes[offset + 1])),
            expand_nibble(parse_nibble(bytes[offset + 2])),
            255,
        ),

        // #RRGGBB
        6 => (
            parse_byte(bytes, offset),
            parse_byte(bytes, offset + 2),
            parse_byte(bytes, offset + 4),
            255,
        ),

        // #RRGGBBAA
        8 => (
            parse_byte(bytes, offset),
            parse_byte(bytes, offset + 2),
            parse_byte(bytes, offset + 4),
            parse_byte(bytes, offset + 6),
        ),

        _ => panic!("Invalid hex color length"),
    };

    Color::from_argb(a, r, g, b)
}
