// API key management (D14): scoped keys (read/write/admin), rotation and
// revocation behind confirmation dialogs, per-key usage stats with 7-day
// mini-bars. Demo-only local state — real issuance is a backend concern; the
// portal never persists secret material (prefixes + metadata only).

import { useState } from "react";
import { RefreshCw, Trash2 } from "lucide-react";
import { ConfirmDialog } from "../components";
import { useToast } from "../lib/toast";

type Scope = "read" | "write" | "admin";

interface KeyRow {
  id: string;
  label: string;
  prefix: string;
  strategy: string;
  scope: Scope;
  created: string;
  requestsToday: number;
  week: number[]; // 7 daily request counts, oldest first
}

function week(seedScale: number): number[] {
  return Array.from({ length: 7 }, () => Math.round(seedScale * (0.5 + Math.random())));
}

function freshPrefix(): string {
  return `ajna_live_sk_${Math.random().toString(16).slice(2, 6)}…`;
}

const INITIAL: KeyRow[] = [
  {
    id: "1",
    label: "Production (iOS)",
    prefix: "ajna_live_sk_9f2a…",
    strategy: "tenant",
    scope: "write",
    created: "2026-07-01",
    requestsToday: 8_412,
    week: [7100, 8900, 8100, 9400, 8800, 7600, 8412],
  },
  {
    id: "2",
    label: "Staging",
    prefix: "ajna_test_sk_1b7c…",
    strategy: "device",
    scope: "read",
    created: "2026-07-03",
    requestsToday: 640,
    week: [420, 380, 510, 890, 720, 555, 640],
  },
];

function UsageBars({ data }: { data: number[] }) {
  const max = Math.max(...data, 1);
  return (
    <span className="usage-bars" aria-label={`7-day usage, today ${data[data.length - 1]}`}>
      {data.map((v, i) => (
        <span
          key={i}
          className="usage-bar"
          style={{ height: `${Math.max(12, (v / max) * 100)}%` }}
        />
      ))}
    </span>
  );
}

export function ApiKeys() {
  const [keys, setKeys] = useState<KeyRow[]>(INITIAL);
  const [label, setLabel] = useState("");
  const [scope, setScope] = useState<Scope>("read");
  const [rotating, setRotating] = useState<KeyRow | null>(null);
  const [revoking, setRevoking] = useState<KeyRow | null>(null);
  const push = useToast();

  function addKey() {
    if (!label.trim()) return;
    setKeys((prev) => [
      ...prev,
      {
        id: crypto.randomUUID(),
        label: label.trim(),
        prefix: freshPrefix(),
        strategy: "tenant",
        scope,
        created: new Date().toISOString().slice(0, 10),
        requestsToday: 0,
        week: week(0),
      },
    ]);
    push(`Key "${label.trim()}" created (${scope})`, "ok");
    setLabel("");
  }

  function rotate(target: KeyRow) {
    setKeys((prev) =>
      prev.map((k) => (k.id === target.id ? { ...k, prefix: freshPrefix() } : k)),
    );
    push(`Key "${target.label}" rotated — old secret invalidated`, "ok");
  }

  function revoke(target: KeyRow) {
    setKeys((prev) => prev.filter((k) => k.id !== target.id));
    push(`Key "${target.label}" revoked`, "warn");
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
        <span className="hud-coord">SEC//KEYS · DEMO DATA</span>
      </div>

      <div className="card">
        <div className="card-title">Create a key</div>
        <div className="row">
          <div>
            <label htmlFor="label">Label</label>
            <input
              id="label"
              type="text"
              value={label}
              placeholder="e.g. Production (Android)"
              onChange={(e) => setLabel(e.target.value)}
            />
          </div>
          <div>
            <label htmlFor="scope">Scope</label>
            <select id="scope" value={scope} onChange={(e) => setScope(e.target.value as Scope)}>
              <option value="read">read — audit + telemetry only</option>
              <option value="write">write — run verifications</option>
              <option value="admin">admin — keys, webhooks, config</option>
            </select>
          </div>
        </div>
        <button type="button" className="btn" onClick={addKey} disabled={!label.trim()}>
          Generate key
        </button>
      </div>

      <div className="card">
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Label</th>
                <th>Key</th>
                <th>Scope</th>
                <th>Strategy</th>
                <th>Created</th>
                <th>Usage (7d)</th>
                <th>Today</th>
                <th aria-label="Actions"></th>
              </tr>
            </thead>
            <tbody>
              {keys.map((k) => (
                <tr key={k.id}>
                  <td>{k.label}</td>
                  <td>
                    <code>{k.prefix}</code>
                  </td>
                  <td>
                    <span className={`pill ${k.scope === "admin" ? "warn" : "ok"}`}>{k.scope}</span>
                  </td>
                  <td>{k.strategy}</td>
                  <td>{k.created}</td>
                  <td>
                    <UsageBars data={k.week} />
                  </td>
                  <td>{k.requestsToday.toLocaleString()}</td>
                  <td className="key-actions">
                    <button
                      type="button"
                      className="ghost-btn"
                      title={`Rotate ${k.label}`}
                      aria-label={`Rotate ${k.label}`}
                      onClick={() => setRotating(k)}
                    >
                      <RefreshCw size={14} />
                    </button>
                    <button
                      type="button"
                      className="ghost-btn ghost-danger"
                      title={`Revoke ${k.label}`}
                      aria-label={`Revoke ${k.label}`}
                      onClick={() => setRevoking(k)}
                    >
                      <Trash2 size={14} />
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {rotating && (
        <ConfirmDialog
          title={`Rotate "${rotating.label}"?`}
          body={
            <p className="page-subtitle">
              A new secret is issued immediately and the current one stops working.
              Deployed clients using <code>{rotating.prefix}</code> must be updated.
            </p>
          }
          confirmLabel="Rotate key"
          onConfirm={() => rotate(rotating)}
          onClose={() => setRotating(null)}
        />
      )}

      {revoking && (
        <ConfirmDialog
          title={`Revoke "${revoking.label}"?`}
          danger
          body={
            <p className="page-subtitle">
              This permanently disables <code>{revoking.prefix}</code>. Requests signed
              with it will fail with <code>401</code>. This cannot be undone.
            </p>
          }
          confirmLabel="Revoke key"
          onConfirm={() => revoke(revoking)}
          onClose={() => setRevoking(null)}
        />
      )}
    </div>
  );
}
