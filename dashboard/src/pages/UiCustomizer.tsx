// Visual UiConfig editor (D13): native color pickers with a synced hex field
// (schema also allows #AARRGGBB — the text field accepts what the picker
// can't express), instant animated live preview, Copy JSON export.

import { useMemo, useState } from "react";
import { CopyButton } from "../components";
import {
  defaultUiConfig,
  validateUiConfig,
  type OverlayShape,
  type UiConfig,
  type UiMode,
} from "../lib/uiConfig";

// Maps the declarative OverlayShape to a CSS border-radius for the live preview.
const SHAPE_RADIUS: Record<OverlayShape, string> = {
  rounded_rect: "12px",
  rect: "0",
  oval: "50%",
  none: "0",
};

/** #AARRGGBB → #RRGGBB for the native picker; #RRGGBB passes through. */
function pickerValue(hex: string): string {
  if (/^#[0-9a-fA-F]{8}$/.test(hex)) return `#${hex.slice(3)}`;
  return /^#[0-9a-fA-F]{6}$/.test(hex) ? hex : "#000000";
}

function ColorField({
  id,
  label,
  value,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (hex: string) => void;
}) {
  return (
    <div>
      <label htmlFor={id}>{label}</label>
      <div className="color-field">
        <input
          id={id}
          type="color"
          value={pickerValue(value)}
          onChange={(e) => onChange(e.target.value.toUpperCase())}
          aria-label={`${label} picker`}
        />
        <input
          type="text"
          value={value}
          aria-label={`${label} hex value`}
          onChange={(e) => onChange(e.target.value)}
        />
      </div>
    </div>
  );
}

export function UiCustomizer() {
  const [config, setConfig] = useState<UiConfig>(defaultUiConfig());

  const errors = useMemo(() => validateUiConfig(config), [config]);
  const exportJson = useMemo(() => JSON.stringify(config, null, 2), [config]);

  // Immutable updates — never mutate the config object in place.
  const patchTheme = (patch: Partial<UiConfig["theme"]>) =>
    setConfig((c) => ({ ...c, theme: { ...c.theme, ...patch } }));
  const patchOverlay = (patch: Partial<UiConfig["overlay"]>) =>
    setConfig((c) => ({ ...c, overlay: { ...c.overlay, ...patch } }));
  const patchAnimations = (patch: Partial<UiConfig["animations"]>) =>
    setConfig((c) => ({ ...c, animations: { ...c.animations, ...patch } }));
  const patchBranding = (patch: Partial<UiConfig["branding"]>) =>
    setConfig((c) => ({ ...c, branding: { ...c.branding, ...patch } }));

  const headless = config.mode === "headless";
  const pulse = config.animations.enabled && config.animations.scan_pulse && !headless;

  return (
    <div>
      <div className="hud-bar">
        <div>
          <h1 className="page-title">UI Customizer</h1>
          <p className="page-subtitle">
            Declarative capture-UI config. Export is validated by the SDK via{" "}
            <code>ajna_ui_config_validate</code> — one schema, portal to device.
          </p>
        </div>
        <span className="hud-coord">CFG//CAPTURE-UI</span>
      </div>

      <div className="row customizer-grid" style={{ alignItems: "start" }}>
        <div>
          <div className="card">
            <div className="card-title">Mode</div>
            <label htmlFor="mode">Capture UI mode</label>
            <select
              id="mode"
              value={config.mode}
              onChange={(e) => setConfig((c) => ({ ...c, mode: e.target.value as UiMode }))}
            >
              <option value="default">Default — Ajna stock UI</option>
              <option value="custom">Custom — re-skinned stock UI</option>
              <option value="headless">Headless — host renders everything</option>
            </select>
            {headless && (
              <p className="page-subtitle">
                Headless mode renders no Ajna UI; theme values below are ignored on device.
              </p>
            )}
          </div>

          <div className="card">
            <div className="card-title">Theme</div>
            <div className="row">
              <ColorField
                id="primary"
                label="Primary color"
                value={config.theme.primary_color}
                onChange={(hex) => patchTheme({ primary_color: hex })}
              />
              <ColorField
                id="bg"
                label="Background color"
                value={config.theme.background_color}
                onChange={(hex) => patchTheme({ background_color: hex })}
              />
            </div>
            <div className="row">
              <ColorField
                id="success"
                label="Success color"
                value={config.theme.success_color}
                onChange={(hex) => patchTheme({ success_color: hex })}
              />
              <ColorField
                id="error"
                label="Error color"
                value={config.theme.error_color}
                onChange={(hex) => patchTheme({ error_color: hex })}
              />
            </div>
            <label htmlFor="radius">Corner radius (dp) — {config.theme.corner_radius_dp}</label>
            <input
              id="radius"
              type="range"
              min={0}
              max={64}
              value={config.theme.corner_radius_dp}
              onChange={(e) => patchTheme({ corner_radius_dp: Number(e.target.value) })}
            />
          </div>

          <div className="card">
            <div className="card-title">Overlay</div>
            <label htmlFor="shape">Guide shape</label>
            <select
              id="shape"
              value={config.overlay.shape}
              onChange={(e) => patchOverlay({ shape: e.target.value as OverlayShape })}
            >
              <option value="rounded_rect">Rounded rectangle</option>
              <option value="rect">Rectangle</option>
              <option value="oval">Oval</option>
              <option value="none">None</option>
            </select>
            <label htmlFor="mask">Mask opacity — {config.overlay.mask_opacity.toFixed(2)}</label>
            <input
              id="mask"
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={config.overlay.mask_opacity}
              onChange={(e) => patchOverlay({ mask_opacity: Number(e.target.value) })}
            />
            <label htmlFor="pulse-toggle">
              <input
                id="pulse-toggle"
                type="checkbox"
                checked={config.animations.scan_pulse}
                onChange={(e) => patchAnimations({ scan_pulse: e.target.checked })}
                style={{ width: "auto", marginRight: "0.5rem" }}
              />
              Scan pulse animation
            </label>
          </div>

          <div className="card">
            <div className="card-title">Branding</div>
            <label htmlFor="company">Company name</label>
            <input
              id="company"
              type="text"
              value={config.branding.company_name ?? ""}
              onChange={(e) => patchBranding({ company_name: e.target.value || null })}
            />
            <label htmlFor="watermark">
              <input
                id="watermark"
                type="checkbox"
                checked={config.branding.show_ajna_watermark}
                onChange={(e) => patchBranding({ show_ajna_watermark: e.target.checked })}
                style={{ width: "auto", marginRight: "0.5rem" }}
              />
              Show Ajna watermark
            </label>
          </div>
        </div>

        <div className="customizer-side">
          <div className="card">
            <div className="card-title">Live preview</div>
            <div
              className="preview-frame"
              style={{ background: config.theme.background_color }}
            >
              {headless ? (
                <span style={{ color: "var(--text-3)", fontSize: "0.8rem" }}>
                  Headless — no Ajna UI
                </span>
              ) : (
                config.overlay.shape !== "none" && (
                  <div
                    className={`preview-guide ${pulse ? "pulse" : ""}`}
                    style={{
                      borderColor: config.theme.primary_color,
                      borderWidth: `${config.overlay.stroke_width_dp}px`,
                      borderRadius: SHAPE_RADIUS[config.overlay.shape],
                      color: config.theme.primary_color,
                    }}
                  >
                    {config.overlay.show_guide_text ? "Align your document" : ""}
                  </div>
                )
              )}
            </div>
            {config.branding.company_name && !headless && (
              <p className="page-subtitle" style={{ marginTop: "0.75rem" }}>
                Branded for <strong>{config.branding.company_name}</strong>
              </p>
            )}
          </div>

          <div className="card">
            <div className="card-title-row">
              <div className="card-title">
                Export{" "}
                {errors.length === 0 ? (
                  <span className="pill ok">valid</span>
                ) : (
                  <span className="pill bad">{errors.length} error(s)</span>
                )}
              </div>
              <CopyButton text={exportJson} label="Copy JSON" />
            </div>
            {errors.map((err) => (
              <p key={err} className="error-text">
                {err}
              </p>
            ))}
            <pre className="code">{exportJson}</pre>
          </div>
        </div>
      </div>
    </div>
  );
}
