use gpui::{Rgba, hsla};

#[inline]
fn parse_channel_255(s: &str) -> Option<f32> {
    Some((s.trim().parse::<f32>().ok()? / 255.0).clamp(0.0, 1.0))
}

#[inline]
fn parse_percent(s: &str) -> Option<f32> {
    Some(s.trim().trim_end_matches('%').trim().parse::<f32>().ok()? / 100.0)
}

#[inline]
fn parse_hue(s: &str) -> Option<f32> {
    Some(s.trim().parse::<f32>().ok()? / 360.0)
}

#[inline]
fn parse_alpha(s: &str) -> Option<f32> {
    Some(s.trim().parse::<f32>().ok()?.clamp(0.0, 1.0))
}

/// Parse a CSS-style color string into a [`Rgba`].
///
/// Accepted forms:
/// - `"transparent"`, `"#rgb"` / `"#rgba"` / `"#rrggbb"` / `"#rrggbbaa"`
/// - `"rgb(r, g, b)"` / `"rgba(r, g, b, a)"` — r/g/b 0–255; a 0.0–1.0
/// - `"hsl(h, s%, l%)"` / `"hsla(h, s%, l%, a)"` — h degrees; s/l percent; a 0.0–1.0
/// - CSS named colors (e.g. `"red"`, `"tomato"`, `"cornflowerblue"`)
pub(crate) fn parse_color(s: &str) -> Option<Rgba> {
    let s = s.trim();

    if s.starts_with('#') {
        return Rgba::try_from(s).ok();
    }

    if let Some(paren) = s.find('(') {
        let name = s[..paren].trim().to_ascii_lowercase();
        let inner = s[paren + 1..].trim_end_matches(')').trim();
        let parts: Vec<&str> = inner.split(',').collect();

        match name.as_str() {
            "rgb" if parts.len() == 3 => {
                let r = parse_channel_255(parts[0])?;
                let g = parse_channel_255(parts[1])?;
                let b = parse_channel_255(parts[2])?;
                return Some(Rgba { r, g, b, a: 1.0 });
            }
            "rgba" if parts.len() == 4 => {
                let r = parse_channel_255(parts[0])?;
                let g = parse_channel_255(parts[1])?;
                let b = parse_channel_255(parts[2])?;
                let a = parse_alpha(parts[3])?;
                return Some(Rgba { r, g, b, a });
            }
            "hsl" if parts.len() == 3 => {
                let h = parse_hue(parts[0])?;
                let s = parse_percent(parts[1])?;
                let l = parse_percent(parts[2])?;
                return Some(hsla(h, s, l, 1.0).to_rgb());
            }
            "hsla" if parts.len() == 4 => {
                let h = parse_hue(parts[0])?;
                let s = parse_percent(parts[1])?;
                let l = parse_percent(parts[2])?;
                let a = parse_alpha(parts[3])?;
                return Some(hsla(h, s, l, a).to_rgb());
            }
            _ => return None,
        }
    }

    parse_named_color(s)
}

/// Resolve a CSS named color (all 148 CSS Level 4 colors plus `"transparent"`).
fn parse_named_color(name: &str) -> Option<Rgba> {
    let hex: u32 = match name.to_ascii_lowercase().as_str() {
        "transparent" => {
            return Some(Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            });
        }
        "aliceblue" => 0xf0f8ff,
        "antiquewhite" => 0xfaebd7,
        "aqua" | "cyan" => 0x00ffff,
        "aquamarine" => 0x7fffd4,
        "azure" => 0xf0ffff,
        "beige" => 0xf5f5dc,
        "bisque" => 0xffe4c4,
        "black" => 0x000000,
        "blanchedalmond" => 0xffebcd,
        "blue" => 0x0000ff,
        "blueviolet" => 0x8a2be2,
        "brown" => 0xa52a2a,
        "burlywood" => 0xdeb887,
        "cadetblue" => 0x5f9ea0,
        "chartreuse" => 0x7fff00,
        "chocolate" => 0xd2691e,
        "coral" => 0xff7f50,
        "cornflowerblue" => 0x6495ed,
        "cornsilk" => 0xfff8dc,
        "crimson" => 0xdc143c,
        "darkblue" => 0x00008b,
        "darkcyan" => 0x008b8b,
        "darkgoldenrod" => 0xb8860b,
        "darkgray" | "darkgrey" => 0xa9a9a9,
        "darkgreen" => 0x006400,
        "darkkhaki" => 0xbdb76b,
        "darkmagenta" => 0x8b008b,
        "darkolivegreen" => 0x556b2f,
        "darkorange" => 0xff8c00,
        "darkorchid" => 0x9932cc,
        "darkred" => 0x8b0000,
        "darksalmon" => 0xe9967a,
        "darkseagreen" => 0x8fbc8f,
        "darkslateblue" => 0x483d8b,
        "darkslategray" | "darkslategrey" => 0x2f4f4f,
        "darkturquoise" => 0x00ced1,
        "darkviolet" => 0x9400d3,
        "deeppink" => 0xff1493,
        "deepskyblue" => 0x00bfff,
        "dimgray" | "dimgrey" => 0x696969,
        "dodgerblue" => 0x1e90ff,
        "firebrick" => 0xb22222,
        "floralwhite" => 0xfffaf0,
        "forestgreen" => 0x228b22,
        "fuchsia" | "magenta" => 0xff00ff,
        "gainsboro" => 0xdcdcdc,
        "ghostwhite" => 0xf8f8ff,
        "gold" => 0xffd700,
        "goldenrod" => 0xdaa520,
        "gray" | "grey" => 0x808080,
        "green" => 0x008000,
        "greenyellow" => 0xadff2f,
        "honeydew" => 0xf0fff0,
        "hotpink" => 0xff69b4,
        "indianred" => 0xcd5c5c,
        "indigo" => 0x4b0082,
        "ivory" => 0xfffff0,
        "khaki" => 0xf0e68c,
        "lavender" => 0xe6e6fa,
        "lavenderblush" => 0xfff0f5,
        "lawngreen" => 0x7cfc00,
        "lemonchiffon" => 0xfffacd,
        "lightblue" => 0xadd8e6,
        "lightcoral" => 0xf08080,
        "lightcyan" => 0xe0ffff,
        "lightgoldenrodyellow" => 0xfafad2,
        "lightgray" | "lightgrey" => 0xd3d3d3,
        "lightgreen" => 0x90ee90,
        "lightpink" => 0xffb6c1,
        "lightsalmon" => 0xffa07a,
        "lightseagreen" => 0x20b2aa,
        "lightskyblue" => 0x87cefa,
        "lightslategray" | "lightslategrey" => 0x778899,
        "lightsteelblue" => 0xb0c4de,
        "lightyellow" => 0xffffe0,
        "lime" => 0x00ff00,
        "limegreen" => 0x32cd32,
        "linen" => 0xfaf0e6,
        "maroon" => 0x800000,
        "mediumaquamarine" => 0x66cdaa,
        "mediumblue" => 0x0000cd,
        "mediumorchid" => 0xba55d3,
        "mediumpurple" => 0x9370db,
        "mediumseagreen" => 0x3cb371,
        "mediumslateblue" => 0x7b68ee,
        "mediumspringgreen" => 0x00fa9a,
        "mediumturquoise" => 0x48d1cc,
        "mediumvioletred" => 0xc71585,
        "midnightblue" => 0x191970,
        "mintcream" => 0xf5fffa,
        "mistyrose" => 0xffe4e1,
        "moccasin" => 0xffe4b5,
        "navajowhite" => 0xffdead,
        "navy" => 0x000080,
        "oldlace" => 0xfdf5e6,
        "olive" => 0x808000,
        "olivedrab" => 0x6b8e23,
        "orange" => 0xffa500,
        "orangered" => 0xff4500,
        "orchid" => 0xda70d6,
        "palegoldenrod" => 0xeee8aa,
        "palegreen" => 0x98fb98,
        "paleturquoise" => 0xafeeee,
        "palevioletred" => 0xdb7093,
        "papayawhip" => 0xffefd5,
        "peachpuff" => 0xffdab9,
        "peru" => 0xcd853f,
        "pink" => 0xffc0cb,
        "plum" => 0xdda0dd,
        "powderblue" => 0xb0e0e6,
        "purple" => 0x800080,
        "rebeccapurple" => 0x663399,
        "red" => 0xff0000,
        "rosybrown" => 0xbc8f8f,
        "royalblue" => 0x4169e1,
        "saddlebrown" => 0x8b4513,
        "salmon" => 0xfa8072,
        "sandybrown" => 0xf4a460,
        "seagreen" => 0x2e8b57,
        "seashell" => 0xfff5ee,
        "sienna" => 0xa0522d,
        "silver" => 0xc0c0c0,
        "skyblue" => 0x87ceeb,
        "slateblue" => 0x6a5acd,
        "slategray" | "slategrey" => 0x708090,
        "snow" => 0xfffafa,
        "springgreen" => 0x00ff7f,
        "steelblue" => 0x4682b4,
        "tan" => 0xd2b48c,
        "teal" => 0x008080,
        "thistle" => 0xd8bfd8,
        "tomato" => 0xff6347,
        "turquoise" => 0x40e0d0,
        "violet" => 0xee82ee,
        "wheat" => 0xf5deb3,
        "white" => 0xffffff,
        "whitesmoke" => 0xf5f5f5,
        "yellow" => 0xffff00,
        "yellowgreen" => 0x9acd32,
        _ => return None,
    };
    let [_, r, g, b] = hex.to_be_bytes().map(|b| b as f32 / 255.0);
    Some(Rgba { r, g, b, a: 1.0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.005
    }

    // ---- transparent ----

    #[test]
    fn transparent_is_fully_clear() {
        let c = parse_color("transparent").unwrap();
        assert_eq!(c.r, 0.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 0.0);
    }

    // ---- hex forms ----

    #[test]
    fn hex_3digit_red() {
        let c = parse_color("#f00").unwrap();
        assert!(approx_eq(c.r, 1.0));
        assert!(approx_eq(c.g, 0.0));
        assert!(approx_eq(c.b, 0.0));
        assert!(approx_eq(c.a, 1.0));
    }

    #[test]
    fn hex_6digit_blue() {
        let c = parse_color("#0000ff").unwrap();
        assert!(approx_eq(c.r, 0.0));
        assert!(approx_eq(c.g, 0.0));
        assert!(approx_eq(c.b, 1.0));
        assert!(approx_eq(c.a, 1.0));
    }

    #[test]
    fn hex_8digit_preserves_alpha() {
        // #ff000080 → red with ~50% alpha (0x80 / 0xff ≈ 0.502)
        let c = parse_color("#ff000080").unwrap();
        assert!(approx_eq(c.r, 1.0));
        assert!(c.a > 0.49 && c.a < 0.51, "alpha was {}", c.a);
    }

    // ---- rgb / rgba ----

    #[test]
    fn rgb_255_0_0_is_red() {
        let c = parse_color("rgb(255, 0, 0)").unwrap();
        assert!(approx_eq(c.r, 1.0));
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn rgba_with_half_alpha() {
        let c = parse_color("rgba(0, 255, 0, 0.5)").unwrap();
        assert_eq!(c.r, 0.0);
        assert!(approx_eq(c.g, 1.0));
        assert_eq!(c.b, 0.0);
        assert!(approx_eq(c.a, 0.5));
    }

    #[test]
    fn rgba_alpha_clamped_to_1() {
        let c = parse_color("rgba(0, 0, 255, 1.5)").unwrap();
        assert!(approx_eq(c.a, 1.0));
    }

    #[test]
    fn rgb_wrong_arg_count_returns_none() {
        assert!(parse_color("rgb(255, 0)").is_none());
    }

    #[test]
    fn rgba_wrong_arg_count_returns_none() {
        assert!(parse_color("rgba(255, 0, 0)").is_none());
    }

    // ---- hsl / hsla ----

    #[test]
    fn hsl_0_100_50_is_red() {
        let c = parse_color("hsl(0, 100%, 50%)").unwrap();
        assert!(c.r > 0.9, "expected red, got r={}", c.r);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn hsla_sets_alpha() {
        let c = parse_color("hsla(0, 100%, 50%, 0.25)").unwrap();
        assert!(approx_eq(c.a, 0.25), "alpha was {}", c.a);
    }

    // ---- named colors ----

    #[test]
    fn named_red() {
        let c = parse_color("red").unwrap();
        assert!(approx_eq(c.r, 1.0));
        assert!(approx_eq(c.g, 0.0));
        assert!(approx_eq(c.b, 0.0));
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn named_cyan_and_aqua_are_identical() {
        let cyan = parse_color("cyan").unwrap();
        let aqua = parse_color("aqua").unwrap();
        assert_eq!(cyan.r, aqua.r);
        assert_eq!(cyan.g, aqua.g);
        assert_eq!(cyan.b, aqua.b);
    }

    #[test]
    fn named_color_case_insensitive() {
        assert!(parse_color("RED").is_some());
        assert!(parse_color("Tomato").is_some());
        assert!(parse_color("cornflowerblue").is_some());
    }

    #[test]
    fn named_color_leading_trailing_whitespace() {
        assert!(parse_color("  red  ").is_some());
    }

    // ---- error paths ----

    #[test]
    fn empty_string_returns_none() {
        assert!(parse_color("").is_none());
    }

    #[test]
    fn unknown_name_returns_none() {
        assert!(parse_color("notacolor").is_none());
        assert!(parse_color("reddish").is_none());
    }

    #[test]
    fn unknown_function_returns_none() {
        assert!(parse_color("lab(50%, 20, -30)").is_none());
    }
}
