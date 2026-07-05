// Mirror of the Rust `UiConfig` (core/src/ui_config.rs). The customizer edits
// this shape and exports JSON that the SDK's `ajna_ui_config_validate` accepts,
// so the portal and the on-device engine agree on one schema.

export type UiMode = "default" | "custom" | "headless";
export type OverlayShape = "rounded_rect" | "rect" | "oval" | "none";

export interface UiTheme {
  primary_color: string;
  background_color: string;
  success_color: string;
  error_color: string;
  corner_radius_dp: number;
  font_family: string | null;
}

export interface UiOverlay {
  shape: OverlayShape;
  stroke_width_dp: number;
  mask_opacity: number;
  show_guide_text: boolean;
}

export interface UiAnimations {
  enabled: boolean;
  scan_pulse: boolean;
  success_checkmark: boolean;
  duration_ms: number;
}

export interface UiBranding {
  company_name: string | null;
  logo_asset: string | null;
  show_ajna_watermark: boolean;
}

export interface UiConfig {
  mode: UiMode;
  theme: UiTheme;
  overlay: UiOverlay;
  animations: UiAnimations;
  branding: UiBranding;
  strings: Record<string, string>;
}

// Same defaults as `UiConfig::default()` in Rust.
export function defaultUiConfig(): UiConfig {
  return {
    mode: "default",
    theme: {
      primary_color: "#6C4DF5",
      background_color: "#0A0A14",
      success_color: "#22C55E",
      error_color: "#EF4444",
      corner_radius_dp: 16,
      font_family: null,
    },
    overlay: {
      shape: "rounded_rect",
      stroke_width_dp: 2,
      mask_opacity: 0.6,
      show_guide_text: true,
    },
    animations: {
      enabled: true,
      scan_pulse: true,
      success_checkmark: true,
      duration_ms: 300,
    },
    branding: {
      company_name: null,
      logo_asset: null,
      show_ajna_watermark: true,
    },
    strings: {},
  };
}

const HEX_COLOR = /^#(?:[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/;

// Client-side mirror of `UiConfig::validate` — same bounds as the Rust core,
// so the customizer flags a bad theme before it is ever shipped to a device.
export function validateUiConfig(config: UiConfig): string[] {
  const errors: string[] = [];
  const colors: [string, string][] = [
    ["primary_color", config.theme.primary_color],
    ["background_color", config.theme.background_color],
    ["success_color", config.theme.success_color],
    ["error_color", config.theme.error_color],
  ];
  for (const [name, value] of colors) {
    if (!HEX_COLOR.test(value)) errors.push(`theme.${name} must be #RRGGBB or #AARRGGBB`);
  }
  if (config.theme.corner_radius_dp < 0 || config.theme.corner_radius_dp > 64) {
    errors.push("theme.corner_radius_dp must be within 0..64");
  }
  if (config.overlay.stroke_width_dp < 0 || config.overlay.stroke_width_dp > 32) {
    errors.push("overlay.stroke_width_dp must be within 0..32");
  }
  if (config.overlay.mask_opacity < 0 || config.overlay.mask_opacity > 1) {
    errors.push("overlay.mask_opacity must be within 0..1");
  }
  if (config.animations.duration_ms > 10000) {
    errors.push("animations.duration_ms must be <= 10000");
  }
  return errors;
}
