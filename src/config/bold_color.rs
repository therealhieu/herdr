use serde::{de, Deserialize, Deserializer};

/// How the client recolors bold text, mirroring Ghostty's `bold-color`.
///
/// Accepted values are `"bright"` or a `"#rrggbb"` hex color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoldColorConfig {
    /// Bold palette colors 0-7 render as their bright variants 8-15.
    Bright,
    /// Like `Bright`, and bold default-foreground text renders in this color.
    Rgb(u8, u8, u8),
}

impl BoldColorConfig {
    fn parse(value: &str) -> Option<Self> {
        if value == "bright" {
            return Some(Self::Bright);
        }
        let hex = value.strip_prefix('#')?;
        if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let channel = |start: usize| u8::from_str_radix(&hex[start..start + 2], 16).ok();
        Some(Self::Rgb(channel(0)?, channel(2)?, channel(4)?))
    }
}

impl<'de> Deserialize<'de> for BoldColorConfig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).ok_or_else(|| {
            de::Error::invalid_value(de::Unexpected::Str(&value), &"\"bright\" or \"#rrggbb\"")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn parse_ui(value: &str) -> Result<Config, toml::de::Error> {
        toml::from_str(&format!("[ui]\nbold_color = {value}\n"))
    }

    #[test]
    fn bold_color_parses_hex_rgb() {
        let config = parse_ui("\"#f8dea6\"").expect("valid bold_color");
        assert_eq!(
            config.ui.bold_color,
            Some(BoldColorConfig::Rgb(248, 222, 166))
        );
    }

    #[test]
    fn bold_color_parses_bright() {
        let config = parse_ui("\"bright\"").expect("valid bold_color");
        assert_eq!(config.ui.bold_color, Some(BoldColorConfig::Bright));
    }

    #[test]
    fn bold_color_defaults_to_none() {
        let config: Config = toml::from_str("[ui]\n").expect("empty ui section");
        assert_eq!(config.ui.bold_color, None);
    }

    #[test]
    fn bold_color_rejects_invalid_values() {
        for value in ["\"yellow\"", "\"#f8dea\""] {
            assert!(
                parse_ui(value).is_err(),
                "{value} must fail the [ui] section"
            );
        }
    }
}
