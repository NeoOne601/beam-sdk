import { useMemo, useState } from "react";
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

export function UiCustomizer() {
  const [config, setConfig] = useState<UiConfig>(defaultUiConfig());

  const errors = useMemo(() => validateUiConfig(config), [config]);
  const exportJson = useMemo(() => JSON.stringify(config, null, 2), [config]);

  // Immutable updates — never mutate the config object in place.
  const patchTheme = (patch: Partial<UiConfig["theme"]>) =>
    setConfig((c) => ({ ...c, theme: { ...c.theme, ...patch } }));
  const patchOverlay = (patch: Partial<UiConfig["overlay"]>) =>
    setConfig((c) => ({ ...c, overlay: { ...c.overlay, ...patch } }));
  const patchBranding = (patch: Partial<UiConfig["branding"]>) =>
    setConfig((c) => ({ ...c, branding: { ...c.branding, ...patch } }));

  const headless = config.mode === "headless";

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

      <div className="row" style={{ gridTemplateColumns: "1fr 300px", alignItems: "start" }}>
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
              <div>
                <label htmlFor="primary">Primary color</label>
                <input
                  id="primary"
                  type="text"
                  value={config.theme.primary_color}
                  onChange={(e) => patchTheme({ primary_color: e.target.value })}
                />
              </div>
              <div>
                <label htmlFor="bg">Background color</label>
                <input
                  id="bg"
                  type="text"
                  value={config.theme.background_color}
                  onChange={(e) => patchTheme({ background_color: e.target.value })}
                />
              </div>
            </div>
            <label htmlFor="radius">Corner radius (dp)</label>
            <input
              id="radius"
              type="number"
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
            <label htmlFor="mask">Mask opacity (0–1)</label>
            <input
              id="mask"
              type="number"
              step="0.05"
              value={config.overlay.mask_opacity}
              onChange={(e) => patchOverlay({ mask_opacity: Number(e.target.value) })}
            />
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

        <div style={{ position: "sticky", top: "1rem" }}>
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
                    className="preview-guide"
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
            <div className="card-title">
              Export{" "}
              {errors.length === 0 ? (
                <span className="pill ok">valid</span>
              ) : (
                <span className="pill bad">{errors.length} error(s)</span>
              )}
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
