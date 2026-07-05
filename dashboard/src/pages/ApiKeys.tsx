import { useState } from "react";

interface KeyRow {
  id: string;
  label: string;
  prefix: string;
  strategy: string;
  created: string;
}

// Demo-only local state — real key issuance is a backend concern. The portal
// never persists secret material; it shows prefixes and metadata only.
const INITIAL: KeyRow[] = [
  {
    id: "1",
    label: "Production (iOS)",
    prefix: "ajna_live_sk_9f2a…",
    strategy: "tenant",
    created: "2026-07-01",
  },
  {
    id: "2",
    label: "Staging",
    prefix: "ajna_test_sk_1b7c…",
    strategy: "device",
    created: "2026-07-03",
  },
];

export function ApiKeys() {
  const [keys, setKeys] = useState<KeyRow[]>(INITIAL);
  const [label, setLabel] = useState("");

  function addKey() {
    if (!label.trim()) return;
    const suffix = Math.random().toString(16).slice(2, 6);
    setKeys((prev) => [
      ...prev,
      {
        id: crypto.randomUUID(),
        label: label.trim(),
        prefix: `ajna_live_sk_${suffix}…`,
        strategy: "tenant",
        created: new Date().toISOString().slice(0, 10),
      },
    ]);
    setLabel("");
  }

  return (
    <div>
      <div className="hud-bar">
        <div>
          <h1 className="page-title">API Keys</h1>
          <p className="page-subtitle">
            Keys map to the backend's <code>KEY_PROVIDER_STRATEGY</code> (tenant / device / model).
            Secret material is shown once at creation and never stored by the portal.
          </p>
        </div>
        <span className="hud-coord">SEC//KEYS</span>
      </div>

      <div className="card">
        <div className="card-title">Create a key</div>
        <label htmlFor="label">Label</label>
        <input
          id="label"
          type="text"
          value={label}
          placeholder="e.g. Production (Android)"
          onChange={(e) => setLabel(e.target.value)}
        />
        <button className="btn" onClick={addKey} disabled={!label.trim()}>
          Generate key
        </button>
      </div>

      <div className="card">
        <table>
          <thead>
            <tr>
              <th>Label</th>
              <th>Key</th>
              <th>Strategy</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            {keys.map((k) => (
              <tr key={k.id}>
                <td>{k.label}</td>
                <td>
                  <code>{k.prefix}</code>
                </td>
                <td>{k.strategy}</td>
                <td>{k.created}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
