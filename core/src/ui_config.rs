// core/src/ui_config.rs
// Declarative capture-UI configuration — the client customization layer.
//
// The Rust core never renders UI. This module is the single, validated
// source of truth that platform shells (Swift / Kotlin / TS) read to draw
// the capture screen — or skip entirely in headless mode. Client businesses
// supply a JSON document (authored in the Ajna dashboard UI Customizer or
// their own config pipeline); the core validates it once at session start
// via `UiConfig::from_json` or the `ajna_ui_config_validate` FFI entry.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Upper bounds enforced by `UiConfig::validate`. These are product policy
/// limits, not technical ones: they keep a hostile or buggy config from
/// degrading the capture experience.
const MAX_STRING_OVERRIDES: usize = 256;
const MAX_STRING_VALUE_LEN: usize = 512;
const MAX_NAME_LEN: usize = 256;
const MAX_ANIMATION_DURATION_MS: u32 = 10_000;
const MAX_CORNER_RADIUS_DP: f32 = 64.0;
const MAX_STROKE_WIDTH_DP: f32 = 32.0;

/// How the host application wants the capture UI driven.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiMode {
    /// Ajna's stock capture UI with the theme applied.
    #[default]
    Default,
    /// Stock capture structure, fully re-skinned via `theme` / `overlay` /
    /// `branding` — the standard white-label path.
    Custom,
    /// No UI at all. The host feeds frames programmatically and renders its
    /// own experience (see `ajna-idv`'s `HeadlessScanner`).
    Headless,
}

/// Shape of the document-guide overlay drawn over the camera preview.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayShape {
    #[default]
    RoundedRect,
    Rect,
    Oval,
    /// No guide overlay (host draws its own inside Custom mode).
    None,
}

/// Color and typography tokens. Colors are `#RRGGBB` or `#AARRGGBB`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiTheme {
    pub primary_color: String,
    pub background_color: String,
    pub success_color: String,
    pub error_color: String,
    pub corner_radius_dp: f32,
    pub font_family: Option<String>,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            primary_color: "#6C4DF5".to_owned(),
            background_color: "#0A0A14".to_owned(),
            success_color: "#22C55E".to_owned(),
            error_color: "#EF4444".to_owned(),
            corner_radius_dp: 16.0,
            font_family: None,
        }
    }
}

/// Document-guide overlay configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiOverlay {
    pub shape: OverlayShape,
    pub stroke_width_dp: f32,
    /// Opacity of the dimmed mask outside the guide, `0.0..=1.0`.
    pub mask_opacity: f32,
    pub show_guide_text: bool,
}

impl Default for UiOverlay {
    fn default() -> Self {
        Self {
            shape: OverlayShape::RoundedRect,
            stroke_width_dp: 2.0,
            mask_opacity: 0.6,
            show_guide_text: true,
        }
    }
}

/// Capture-screen animation switches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiAnimations {
    pub enabled: bool,
    pub scan_pulse: bool,
    pub success_checkmark: bool,
    pub duration_ms: u32,
}

impl Default for UiAnimations {
    fn default() -> Self {
        Self {
            enabled: true,
            scan_pulse: true,
            success_checkmark: true,
            duration_ms: 300,
        }
    }
}

/// Client branding assets.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiBranding {
    pub company_name: Option<String>,
    /// Host-resolvable asset identifier (bundle resource name / URL).
    pub logo_asset: Option<String>,
    /// White-label switch; defaults to showing the Ajna mark.
    pub show_ajna_watermark: bool,
}

/// The full declarative UI configuration document.
///
/// Every section defaults, so `{}` is a valid config (stock UI). Unknown
/// fields are ignored for forward compatibility with newer dashboards.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub mode: UiMode,
    pub theme: UiTheme,
    pub overlay: UiOverlay,
    pub animations: UiAnimations,
    pub branding: UiBranding,
    /// String overrides keyed by well-known slot (e.g. `"scan_prompt"`).
    /// BTreeMap keeps serialization deterministic.
    pub strings: BTreeMap<String, String>,
}

/// Validation failures for a declarative UI configuration.
#[derive(Debug, Clone, PartialEq)]
pub enum UiConfigError {
    Json(String),
    InvalidColor { field: &'static str, value: String },
    OutOfRange { field: &'static str },
}

impl fmt::Display for UiConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(msg) => write!(f, "invalid UI config JSON: {msg}"),
            Self::InvalidColor { field, value } => {
                write!(
                    f,
                    "invalid color for {field}: {value:?} (expected #RRGGBB or #AARRGGBB)"
                )
            }
            Self::OutOfRange { field } => write!(f, "value out of range for {field}"),
        }
    }
}

impl std::error::Error for UiConfigError {}

impl UiConfig {
    /// A configuration that suppresses all Ajna-rendered UI.
    pub fn headless() -> Self {
        Self {
            mode: UiMode::Headless,
            ..Self::default()
        }
    }

    pub fn is_headless(&self) -> bool {
        self.mode == UiMode::Headless
    }

    /// Parse and validate a JSON document.
    pub fn from_json(json: &str) -> Result<Self, UiConfigError> {
        let config: Self =
            serde_json::from_str(json).map_err(|e| UiConfigError::Json(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// Serialize to JSON. Infallible: the struct contains only string-keyed
    /// maps and plain data, which serde_json always serializes.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("UiConfig JSON serialization cannot fail")
    }

    /// Enforce color syntax and numeric bounds. Called by `from_json`;
    /// exposed for configs constructed in Rust.
    pub fn validate(&self) -> Result<(), UiConfigError> {
        validate_color("theme.primary_color", &self.theme.primary_color)?;
        validate_color("theme.background_color", &self.theme.background_color)?;
        validate_color("theme.success_color", &self.theme.success_color)?;
        validate_color("theme.error_color", &self.theme.error_color)?;

        if !(0.0..=MAX_CORNER_RADIUS_DP).contains(&self.theme.corner_radius_dp) {
            return Err(UiConfigError::OutOfRange {
                field: "theme.corner_radius_dp",
            });
        }
        if !(0.0..=MAX_STROKE_WIDTH_DP).contains(&self.overlay.stroke_width_dp) {
            return Err(UiConfigError::OutOfRange {
                field: "overlay.stroke_width_dp",
            });
        }
        if !(0.0..=1.0).contains(&self.overlay.mask_opacity) {
            return Err(UiConfigError::OutOfRange {
                field: "overlay.mask_opacity",
            });
        }
        if self.animations.duration_ms > MAX_ANIMATION_DURATION_MS {
            return Err(UiConfigError::OutOfRange {
                field: "animations.duration_ms",
            });
        }
        if let Some(name) = &self.branding.company_name {
            if name.len() > MAX_NAME_LEN {
                return Err(UiConfigError::OutOfRange {
                    field: "branding.company_name",
                });
            }
        }
        if let Some(asset) = &self.branding.logo_asset {
            if asset.len() > MAX_NAME_LEN {
                return Err(UiConfigError::OutOfRange {
                    field: "branding.logo_asset",
                });
            }
        }
        if let Some(font) = &self.theme.font_family {
            if font.len() > MAX_NAME_LEN {
                return Err(UiConfigError::OutOfRange {
                    field: "theme.font_family",
                });
            }
        }
        if self.strings.len() > MAX_STRING_OVERRIDES {
            return Err(UiConfigError::OutOfRange { field: "strings" });
        }
        for (key, value) in &self.strings {
            if key.is_empty() || value.len() > MAX_STRING_VALUE_LEN {
                return Err(UiConfigError::OutOfRange { field: "strings" });
            }
        }
        Ok(())
    }
}

fn validate_color(field: &'static str, value: &str) -> Result<(), UiConfigError> {
    let hex = match value.strip_prefix('#') {
        Some(h) if h.len() == 6 || h.len() == 8 => h,
        _ => {
            return Err(UiConfigError::InvalidColor {
                field,
                value: value.to_owned(),
            });
        }
    };
    if hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(UiConfigError::InvalidColor {
            field,
            value: value.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid_and_not_headless() {
        let config = UiConfig::default();
        assert!(config.validate().is_ok());
        assert!(!config.is_headless());
        assert_eq!(config.mode, UiMode::Default);
    }

    #[test]
    fn empty_json_document_is_the_stock_config() {
        let config = UiConfig::from_json("{}").expect("empty object must parse");
        assert_eq!(config, UiConfig::default());
    }

    #[test]
    fn headless_roundtrips_through_json() {
        let json = UiConfig::headless().to_json();
        let parsed = UiConfig::from_json(&json).expect("roundtrip must parse");
        assert!(parsed.is_headless());
    }

    #[test]
    fn rejects_malformed_color() {
        let json = r##"{"theme": {"primary_color": "purple"}}"##;
        let err = UiConfig::from_json(json).expect_err("non-hex color must fail");
        assert!(matches!(
            err,
            UiConfigError::InvalidColor {
                field: "theme.primary_color",
                ..
            }
        ));
    }

    #[test]
    fn accepts_argb_hex_color() {
        let json = r##"{"theme": {"primary_color": "#80FF00AA"}}"##;
        assert!(UiConfig::from_json(json).is_ok());
    }

    #[test]
    fn rejects_mask_opacity_above_one() {
        let json = r#"{"overlay": {"mask_opacity": 1.5}}"#;
        let err = UiConfig::from_json(json).expect_err("opacity > 1 must fail");
        assert_eq!(
            err,
            UiConfigError::OutOfRange {
                field: "overlay.mask_opacity"
            }
        );
    }

    #[test]
    fn rejects_invalid_json_syntax() {
        assert!(matches!(
            UiConfig::from_json("{nope"),
            Err(UiConfigError::Json(_))
        ));
    }

    #[test]
    fn unknown_fields_are_ignored_for_forward_compat() {
        let json = r#"{"future_section": {"x": 1}, "mode": "custom"}"#;
        let config = UiConfig::from_json(json).expect("unknown fields must be tolerated");
        assert_eq!(config.mode, UiMode::Custom);
    }

    #[test]
    fn white_label_branding_parses() {
        let json = r##"{
            "mode": "custom",
            "theme": {"primary_color": "#FF5733"},
            "branding": {"company_name": "Acme Bank", "show_ajna_watermark": false},
            "strings": {"scan_prompt": "Scan the front of your Acme ID"}
        }"##;
        let config = UiConfig::from_json(json).expect("white-label config must parse");
        assert_eq!(config.branding.company_name.as_deref(), Some("Acme Bank"));
        assert!(!config.branding.show_ajna_watermark);
        assert_eq!(
            config.strings.get("scan_prompt").map(String::as_str),
            Some("Scan the front of your Acme ID")
        );
    }
}
