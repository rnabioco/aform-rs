//! UI theme colors for the application.
//!
//! This module defines all UI element colors that can be customized via config.

use std::fmt;

use ratatui::style::Color;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// RGB color representation for config serialization.
///
/// Supports multiple input formats:
/// - Hex: `"#FF8000"` or `"#ff8000"`
/// - RGB string: `"255,128,0"`
/// - Verbose: `{ r = 255, g = 128, b = 0 }`
#[derive(Debug, Clone, Copy)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn to_color(self) -> Color {
        Color::Rgb(self.r, self.g, self.b)
    }

    /// Parse from hex string like "#FF8000" or "FF8000"
    fn from_hex(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if s.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(Self { r, g, b })
    }

    /// Parse from comma-separated string like "255,128,0"
    fn from_csv(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
        if parts.len() != 3 {
            return None;
        }
        let r = parts[0].parse().ok()?;
        let g = parts[1].parse().ok()?;
        let b = parts[2].parse().ok()?;
        Some(Self { r, g, b })
    }
}

impl From<Rgb> for Color {
    fn from(rgb: Rgb) -> Self {
        rgb.to_color()
    }
}

impl Serialize for Rgb {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Output as hex format "#RRGGBB"
        serializer.serialize_str(&format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b))
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RgbVisitor;

        impl<'de> Visitor<'de> for RgbVisitor {
            type Value = Rgb;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a color as hex \"#RRGGBB\", CSV \"r,g,b\", or map { r, g, b }")
            }

            // Handle string formats: "#RRGGBB" or "r,g,b"
            fn visit_str<E>(self, value: &str) -> Result<Rgb, E>
            where
                E: de::Error,
            {
                if value.starts_with('#') || value.chars().all(|c| c.is_ascii_hexdigit()) {
                    Rgb::from_hex(value).ok_or_else(|| {
                        de::Error::invalid_value(
                            de::Unexpected::Str(value),
                            &"hex color like #FF8000",
                        )
                    })
                } else if value.contains(',') {
                    Rgb::from_csv(value).ok_or_else(|| {
                        de::Error::invalid_value(
                            de::Unexpected::Str(value),
                            &"RGB values like 255,128,0",
                        )
                    })
                } else {
                    Err(de::Error::invalid_value(
                        de::Unexpected::Str(value),
                        &"hex (#FF8000) or CSV (255,128,0) color format",
                    ))
                }
            }

            // Handle map format: { r = X, g = Y, b = Z }
            fn visit_map<M>(self, mut map: M) -> Result<Rgb, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut r = None;
                let mut g = None;
                let mut b = None;

                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "r" => r = Some(map.next_value()?),
                        "g" => g = Some(map.next_value()?),
                        "b" => b = Some(map.next_value()?),
                        _ => {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                Ok(Rgb {
                    r: r.ok_or_else(|| de::Error::missing_field("r"))?,
                    g: g.ok_or_else(|| de::Error::missing_field("g"))?,
                    b: b.ok_or_else(|| de::Error::missing_field("b"))?,
                })
            }
        }

        deserializer.deserialize_any(RgbVisitor)
    }
}

/// Border colors for panes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BorderColors {
    pub active: Rgb,
    pub inactive: Rgb,
}

impl Default for BorderColors {
    fn default() -> Self {
        Self {
            active: Rgb::new(0, 255, 255),     // Cyan
            inactive: Rgb::new(128, 128, 128), // DarkGray
        }
    }
}

impl BorderColors {
    /// Light mode defaults with darker colors for visibility on light backgrounds.
    pub fn default_for_light() -> Self {
        Self {
            active: Rgb::new(0, 0, 180),    // Dark blue
            inactive: Rgb::new(100, 100, 100), // Gray
        }
    }
}

/// Ruler display colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RulerColors {
    pub numbers: Rgb,
    pub ticks: Rgb,
    pub pair_line: Rgb,
}

impl Default for RulerColors {
    fn default() -> Self {
        Self {
            numbers: Rgb::new(128, 128, 128), // DarkGray
            ticks: Rgb::new(128, 128, 128),   // DarkGray
            pair_line: Rgb::new(255, 0, 255), // Magenta
        }
    }
}

impl RulerColors {
    /// Light mode defaults with darker colors for visibility on light backgrounds.
    pub fn default_for_light() -> Self {
        Self {
            numbers: Rgb::new(80, 80, 80),    // Gray
            ticks: Rgb::new(80, 80, 80),      // Gray
            pair_line: Rgb::new(180, 0, 180), // Darker magenta
        }
    }
}

/// Status bar mode indicator colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModeColors {
    pub normal_bg: Rgb,
    pub normal_fg: Rgb,
    pub insert_bg: Rgb,
    pub insert_fg: Rgb,
    pub command_bg: Rgb,
    pub command_fg: Rgb,
    pub search_bg: Rgb,
    pub search_fg: Rgb,
    pub visual_bg: Rgb,
    pub visual_fg: Rgb,
}

impl Default for ModeColors {
    fn default() -> Self {
        Self {
            normal_bg: Rgb::new(0, 0, 255),     // Blue
            normal_fg: Rgb::new(255, 255, 255), // White
            insert_bg: Rgb::new(0, 128, 0),     // Green
            insert_fg: Rgb::new(0, 0, 0),       // Black
            command_bg: Rgb::new(255, 255, 0),  // Yellow
            command_fg: Rgb::new(0, 0, 0),      // Black
            search_bg: Rgb::new(255, 0, 255),   // Magenta
            search_fg: Rgb::new(255, 255, 255), // White
            visual_bg: Rgb::new(100, 100, 180), // Purple-ish
            visual_fg: Rgb::new(255, 255, 255), // White
        }
    }
}

impl ModeColors {
    /// Light mode defaults - same colors work well on both themes.
    pub fn default_for_light() -> Self {
        Self::default()
    }
}

/// Status bar colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusBarColors {
    pub background: Rgb,
    pub position: Rgb,
    pub alignment_info: Rgb,
    pub sequence_type: Rgb,
    pub color_scheme: Rgb,
    pub structure_info: Rgb,
    pub selection_info: Rgb,
    #[serde(flatten)]
    pub modes: ModeColors,
}

impl Default for StatusBarColors {
    fn default() -> Self {
        Self {
            background: Rgb::new(128, 128, 128),     // DarkGray
            position: Rgb::new(255, 255, 255),       // White (default)
            alignment_info: Rgb::new(0, 255, 255),   // Cyan
            sequence_type: Rgb::new(0, 128, 0),      // Green
            color_scheme: Rgb::new(255, 0, 255),     // Magenta
            structure_info: Rgb::new(255, 255, 0),   // Yellow
            selection_info: Rgb::new(173, 216, 230), // LightBlue
            modes: ModeColors::default(),
        }
    }
}

impl StatusBarColors {
    /// Light mode defaults with lighter background and darker text colors.
    pub fn default_for_light() -> Self {
        Self {
            background: Rgb::new(200, 200, 200),  // LightGray
            position: Rgb::new(0, 0, 0),          // Black
            alignment_info: Rgb::new(0, 100, 150), // Dark cyan
            sequence_type: Rgb::new(0, 100, 0),   // Dark green
            color_scheme: Rgb::new(150, 0, 150),  // Dark magenta
            structure_info: Rgb::new(180, 140, 0), // Dark yellow/gold
            selection_info: Rgb::new(0, 80, 120), // Dark blue
            modes: ModeColors::default_for_light(),
        }
    }
}

/// ID column colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IdColumnColors {
    pub text: Rgb,
    pub selected_bg: Rgb,
    pub selected_fg: Rgb,
}

impl Default for IdColumnColors {
    fn default() -> Self {
        Self {
            text: Rgb::new(0, 255, 255),          // Cyan
            selected_bg: Rgb::new(80, 80, 140),   // Purple-ish
            selected_fg: Rgb::new(255, 255, 255), // White
        }
    }
}

impl IdColumnColors {
    /// Light mode defaults with dark text colors for visibility on light backgrounds.
    pub fn default_for_light() -> Self {
        Self {
            text: Rgb::new(0, 0, 139),             // Dark blue
            selected_bg: Rgb::new(180, 180, 220),  // Light purple
            selected_fg: Rgb::new(0, 0, 0),        // Black
        }
    }
}

/// Annotation bar colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnnotationColors {
    pub ss_cons_fg: Rgb,
    pub ss_cons_bg: Rgb,
    pub ss_cons_paired_fg: Rgb,
    pub ss_cons_paired_bg: Rgb,
    pub rf_conserved_fg: Rgb,
    pub rf_conserved_bg: Rgb,
    pub rf_variable_fg: Rgb,
    pub rf_variable_bg: Rgb,
    pub pp_cons_bg: Rgb,
    pub consensus_fg: Rgb,
    pub consensus_bg: Rgb,
    pub conservation_bg: Rgb,
    pub label_ss_cons_fg: Rgb,
    pub label_rf_fg: Rgb,
    pub label_pp_cons_fg: Rgb,
    pub label_consensus_fg: Rgb,
    pub label_conservation_fg: Rgb,
}

impl Default for AnnotationColors {
    fn default() -> Self {
        Self {
            ss_cons_fg: Rgb::new(255, 255, 0), // Yellow
            ss_cons_bg: Rgb::new(30, 30, 40),
            ss_cons_paired_fg: Rgb::new(0, 0, 0),     // Black
            ss_cons_paired_bg: Rgb::new(255, 255, 0), // Yellow
            rf_conserved_fg: Rgb::new(0, 128, 0),     // Green
            rf_conserved_bg: Rgb::new(30, 40, 30),
            rf_variable_fg: Rgb::new(128, 128, 128), // DarkGray
            rf_variable_bg: Rgb::new(30, 30, 30),
            pp_cons_bg: Rgb::new(30, 30, 40),
            consensus_fg: Rgb::new(0, 255, 255), // Cyan
            consensus_bg: Rgb::new(30, 40, 30),
            conservation_bg: Rgb::new(40, 30, 40),
            label_ss_cons_fg: Rgb::new(255, 255, 0), // Yellow
            label_rf_fg: Rgb::new(0, 128, 0),        // Green
            label_pp_cons_fg: Rgb::new(255, 255, 0), // Yellow
            label_consensus_fg: Rgb::new(0, 255, 255), // Cyan
            label_conservation_fg: Rgb::new(255, 0, 255), // Magenta
        }
    }
}

impl AnnotationColors {
    /// Light mode defaults with light-tinted backgrounds and darker text colors.
    pub fn default_for_light() -> Self {
        Self {
            ss_cons_fg: Rgb::new(140, 100, 0),        // Dark gold
            ss_cons_bg: Rgb::new(235, 235, 250),      // Pale blue
            ss_cons_paired_fg: Rgb::new(0, 0, 0),     // Black
            ss_cons_paired_bg: Rgb::new(255, 220, 100), // Light orange/yellow
            rf_conserved_fg: Rgb::new(0, 100, 0),     // Dark green
            rf_conserved_bg: Rgb::new(235, 250, 235), // Pale green
            rf_variable_fg: Rgb::new(100, 100, 100),  // Gray
            rf_variable_bg: Rgb::new(245, 245, 245),  // Very light gray
            pp_cons_bg: Rgb::new(235, 235, 250),      // Pale blue
            consensus_fg: Rgb::new(0, 100, 150),      // Dark cyan
            consensus_bg: Rgb::new(235, 250, 235),    // Pale green
            conservation_bg: Rgb::new(250, 235, 250), // Pale magenta
            label_ss_cons_fg: Rgb::new(140, 100, 0),  // Dark gold
            label_rf_fg: Rgb::new(0, 100, 0),         // Dark green
            label_pp_cons_fg: Rgb::new(140, 100, 0),  // Dark gold
            label_consensus_fg: Rgb::new(0, 100, 150), // Dark cyan
            label_conservation_fg: Rgb::new(150, 0, 150), // Dark magenta
        }
    }
}

/// Selection and highlight colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SelectionColors {
    pub visual_bg: Rgb,
    pub visual_fg: Rgb,
    pub search_current_bg: Rgb,
    pub search_current_fg: Rgb,
    pub search_other_bg: Rgb,
    pub search_other_fg: Rgb,
    pub pair_highlight_bg: Rgb,
    pub pair_highlight_fg: Rgb,
    pub gap_column_bg: Rgb,
}

impl Default for SelectionColors {
    fn default() -> Self {
        Self {
            visual_bg: Rgb::new(80, 80, 140),
            visual_fg: Rgb::new(255, 255, 255),
            search_current_bg: Rgb::new(255, 255, 0), // Yellow
            search_current_fg: Rgb::new(0, 0, 0),     // Black
            search_other_bg: Rgb::new(100, 100, 50),
            search_other_fg: Rgb::new(255, 255, 255),
            pair_highlight_bg: Rgb::new(255, 0, 255), // Magenta
            pair_highlight_fg: Rgb::new(255, 255, 255),
            gap_column_bg: Rgb::new(80, 50, 50), // Dim red
        }
    }
}

impl SelectionColors {
    /// Light mode defaults with lighter backgrounds for visibility.
    pub fn default_for_light() -> Self {
        Self {
            visual_bg: Rgb::new(180, 180, 220),   // Light purple
            visual_fg: Rgb::new(0, 0, 0),         // Black
            search_current_bg: Rgb::new(255, 255, 0), // Yellow (works well)
            search_current_fg: Rgb::new(0, 0, 0),     // Black
            search_other_bg: Rgb::new(220, 220, 150), // Light yellow-ish
            search_other_fg: Rgb::new(0, 0, 0),       // Black
            pair_highlight_bg: Rgb::new(255, 180, 255), // Light magenta
            pair_highlight_fg: Rgb::new(0, 0, 0),      // Black
            gap_column_bg: Rgb::new(250, 220, 220),    // Light red
        }
    }
}

/// Command and search line colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CommandLineColors {
    pub command_prefix: Rgb,
    pub search_prefix: Rgb,
    pub help_hint: Rgb,
}

impl Default for CommandLineColors {
    fn default() -> Self {
        Self {
            command_prefix: Rgb::new(255, 255, 0), // Yellow
            search_prefix: Rgb::new(255, 0, 255),  // Magenta
            help_hint: Rgb::new(128, 128, 128),    // DarkGray
        }
    }
}

impl CommandLineColors {
    /// Light mode defaults with darker colors for visibility.
    pub fn default_for_light() -> Self {
        Self {
            command_prefix: Rgb::new(180, 140, 0), // Dark gold
            search_prefix: Rgb::new(150, 0, 150),  // Dark magenta
            help_hint: Rgb::new(100, 100, 100),    // Gray
        }
    }
}

/// Separator and misc UI colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MiscColors {
    pub separator: Rgb,
    pub tree_dark_theme: Rgb,
    pub tree_light_theme: Rgb,
}

impl Default for MiscColors {
    fn default() -> Self {
        Self {
            separator: Rgb::new(128, 128, 128),       // DarkGray
            tree_dark_theme: Rgb::new(255, 255, 255), // White
            tree_light_theme: Rgb::new(0, 0, 0),      // Black
        }
    }
}

impl MiscColors {
    /// Light mode defaults with darker separator for visibility.
    pub fn default_for_light() -> Self {
        Self {
            separator: Rgb::new(100, 100, 100), // Gray
            tree_dark_theme: Rgb::new(255, 255, 255), // White (unchanged)
            tree_light_theme: Rgb::new(0, 0, 0),      // Black (unchanged)
        }
    }
}

/// Complete UI theme containing all color settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub border: BorderColors,
    pub ruler: RulerColors,
    pub status_bar: StatusBarColors,
    pub id_column: IdColumnColors,
    pub annotations: AnnotationColors,
    pub selection: SelectionColors,
    pub command_line: CommandLineColors,
    pub misc: MiscColors,
}

impl Theme {
    /// Create a theme optimized for light terminal backgrounds.
    pub fn default_for_light() -> Self {
        Self {
            border: BorderColors::default_for_light(),
            ruler: RulerColors::default_for_light(),
            status_bar: StatusBarColors::default_for_light(),
            id_column: IdColumnColors::default_for_light(),
            annotations: AnnotationColors::default_for_light(),
            selection: SelectionColors::default_for_light(),
            command_line: CommandLineColors::default_for_light(),
            misc: MiscColors::default_for_light(),
        }
    }
}
