import { useMemo, useState } from "react";

type Platform = "ios" | "android" | "web";

const SNIPPETS: Record<Platform, (key: string) => string> = {
  ios: (key) => `// Swift — add the Ajna SDK, then:
import AjnaSDK

let ajna = AjnaSDK(apiKey: "${key}")
let session = ajna.startIdvSession(uiConfig: .default)
// Feed camera frames; retrieve the signed result when state == .complete`,
  android: (key) => `// Kotlin — add the Ajna AAR, then:
import com.ajna.sdk.AjnaSDK

val ajna = AjnaSDK(apiKey = "${key}")
val session = ajna.startIdvSession(uiConfig = UiConfig.DEFAULT)
// Feed camera frames; retrieve the signed result when state == COMPLETE`,
  web: (key) => `// TypeScript — npm i @ajna/sdk
import { AjnaScanner } from "@ajna/sdk";

const scanner = new AjnaScanner({ apiKey: "${key}" });
await scanner.start({ mode: "default" });
// Or run headless: await scanner.start({ mode: "headless" })`,
};

const STEPS = ["API Key", "Platform", "Integrate", "Test Call", "Go Live"];

export function Onboarding() {
  const [step, setStep] = useState(0);
  const [apiKey, setApiKey] = useState("ajna_live_sk_demo_0000");
  const [platform, setPlatform] = useState<Platform>("ios");

  const snippet = useMemo(() => SNIPPETS[platform](apiKey), [platform, apiKey]);

  return (
    <div>
      <div className="hud-bar">
        <div>
          <h1 className="page-title">60-Minute Integration</h1>
          <p className="page-subtitle">
            Five steps from zero to a signed, compliant verification in production.
          </p>
        </div>
        <span className="hud-coord">OPS//ONBOARD</span>
      </div>

      <div className="steps">
        {STEPS.map((label, i) => (
          <div
            key={label}
            className={`step ${i === step ? "active" : ""} ${i < step ? "done" : ""}`}
          >
            {i < step ? "✓ " : `${i + 1}. `}
            {label}
          </div>
        ))}
      </div>

      {step === 0 && (
        <div className="card">
          <div className="card-title">Your publishable API key</div>
          <label htmlFor="key">API Key</label>
          <input id="key" type="text" value={apiKey} onChange={(e) => setApiKey(e.target.value)} />
          <p className="page-subtitle">
            Sent as <code>X-Api-Key</code> to the Ajna backend. Keep the secret key server-side.
          </p>
        </div>
      )}

      {step === 1 && (
        <div className="card">
          <div className="card-title">Target platform</div>
          <label htmlFor="platform">Platform</label>
          <select
            id="platform"
            value={platform}
            onChange={(e) => setPlatform(e.target.value as Platform)}
          >
            <option value="ios">iOS (Swift)</option>
            <option value="android">Android (Kotlin)</option>
            <option value="web">Web (TypeScript / WASM)</option>
          </select>
        </div>
      )}

      {step === 2 && (
        <div className="card">
          <div className="card-title">Drop-in integration snippet</div>
          <pre className="code">{snippet}</pre>
        </div>
      )}

      {step === 3 && (
        <div className="card">
          <div className="card-title">Verify the connection</div>
          <p className="page-subtitle">Run this against your backend to confirm auth works:</p>
          <pre className="code">{`curl -s https://api.your-ajna.example/health \\
  -H "X-Api-Key: ${apiKey}"

# Expected: {"status":"ok"}`}</pre>
        </div>
      )}

      {step === 4 && (
        <div className="card">
          <div className="card-title">You're live 🎉</div>
          <p className="page-subtitle">
            IDV, Intel (device posture), and Vision (liveness) share one PQC-signed result
            envelope. Country rules and NQM compliance are enforced server-side automatically.
          </p>
          <span className="pill ok">SOC2 audit logging active</span>{" "}
          <span className="pill ok">NQM PQC signing active</span>
        </div>
      )}

      <div style={{ display: "flex", gap: "0.75rem", marginTop: "0.5rem" }}>
        <button
          className="btn secondary"
          disabled={step === 0}
          onClick={() => setStep((s) => Math.max(0, s - 1))}
        >
          Back
        </button>
        <button
          className="btn"
          disabled={step === STEPS.length - 1}
          onClick={() => setStep((s) => Math.min(STEPS.length - 1, s + 1))}
        >
          {step === STEPS.length - 2 ? "Finish" : "Next"}
        </button>
      </div>
    </div>
  );
}
