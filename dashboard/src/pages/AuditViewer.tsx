// SOC2 audit chain viewer (D12): row click opens a drill-down panel with the
// full payload, PQC signature material, chain position, and event timeline.
// Loads from the live backend; "Load demo data" seeds a simulated chain so the
// drill-down is always demonstrable (free-tier backend sleeps).

import { useEffect, useState } from "react";
import { X } from "lucide-react";
import { AjnaClient, DEFAULT_API_BASE, type AuditEntry, type ChainReport } from "../lib/api";
import { Drawer, Hash, Metric, Skeleton } from "../components";
import { useToast } from "../lib/toast";

const DEMO_EVENTS = [
  ["idv_document_verified", "success"],
  ["vision_liveness_cleared", "success"],
  ["intel_posture_scored", "success"],
  ["idv_document_verified", "failure"],
  ["vision_liveness_timeout", "failure"],
  ["nqm_attestation_issued", "success"],
] as const;

function demoEntries(): AuditEntry[] {
  const now = Date.now();
  return Array.from({ length: 14 }, (_, i) => {
    const [event, outcome] = DEMO_EVENTS[i % DEMO_EVENTS.length];
    return {
      id: `${1400 - i}`,
      session_id: crypto.randomUUID(),
      event_type: event,
      outcome,
      detail: {
        algo: "ML-DSA-65",
        signature_b64: btoa(String.fromCharCode(...crypto.getRandomValues(new Uint8Array(48)))),
        confidence: Math.round((0.82 + Math.random() * 0.17) * 100) / 100,
        country_rules: "IN",
        chain_prev_sha256: Array.from(crypto.getRandomValues(new Uint8Array(32)))
          .map((b) => b.toString(16).padStart(2, "0"))
          .join(""),
      },
      created_at: new Date(now - i * 137_000).toISOString(),
    };
  });
}

export function AuditViewer() {
  const [baseUrl, setBaseUrl] = useState(DEFAULT_API_BASE);
  const [apiKey, setApiKey] = useState("");
  const [entries, setEntries] = useState<AuditEntry[]>([]);
  const [selected, setSelected] = useState<AuditEntry | null>(null);
  const [chain, setChain] = useState<ChainReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [demo, setDemo] = useState(false);
  const push = useToast();

  const client = () => new AjnaClient(baseUrl, apiKey);

  // Escape closes the drill-down panel (D10).
  useEffect(() => {
    if (!selected) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setSelected(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [selected]);

  async function loadLogs() {
    setLoading(true);
    setError(null);
    try {
      const res = await client().auditLogs(50);
      setEntries(res.entries);
      setDemo(false);
      push(`Loaded ${res.entries.length} audit entries`, "ok");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }

  async function checkChain() {
    setError(null);
    try {
      const report = await client().verifyChain();
      setChain(report);
      push(report.valid ? "Chain verified — intact" : "Chain verification FAILED", report.valid ? "ok" : "crit");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  function loadDemo() {
    setEntries(demoEntries());
    setChain({ valid: true, entries_checked: 1400, legacy_entries_skipped: 3, first_broken_seq: null });
    setDemo(true);
    setError(null);
    push("Demo audit chain loaded (simulated)", "info");
  }

  const detail = selected?.detail as Record<string, unknown> | null | undefined;

  return (
    <div>
      <div className="hud-bar">
        <div>
          <h1 className="page-title">Audit Log</h1>
          <p className="page-subtitle">
            Tamper-evident SOC2 trail. Every verification is SHA-256 hash-chained.
          </p>
        </div>
        <span className="hud-coord">SEC//AUDIT-CHAIN{demo ? " · SIMULATED" : ""}</span>
      </div>

      {/* Connection controls — minimal typing, top of the F. */}
      <div className="card">
        <div className="card-title">Connection</div>
        <div className="row">
          <div>
            <label htmlFor="url">Backend URL</label>
            <input id="url" type="text" value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} />
          </div>
          <div>
            <label htmlFor="key">API Key</label>
            <input
              id="key"
              type="password"
              autoComplete="off"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
            />
          </div>
        </div>
        <div style={{ display: "flex", gap: "0.6rem", flexWrap: "wrap" }}>
          <button type="button" className="btn" onClick={loadLogs} disabled={loading}>
            {loading ? "Loading…" : "Load entries"}
          </button>
          <button type="button" className="btn secondary" onClick={checkChain}>
            Verify chain
          </button>
          <button type="button" className="btn secondary" onClick={loadDemo}>
            Load demo data
          </button>
        </div>
        {error && <p className="error-text" style={{ marginTop: "0.6rem" }}>{error}</p>}
      </div>

      {loading && (
        <div className="card">
          <div className="card-title">Fetching audit trail…</div>
          <Skeleton lines={5} />
        </div>
      )}

      {/* Chain integrity summary — HUD metrics + collapsible detail. */}
      {chain && !loading && (
        <div className="card enter">
          <div className="card-title">Chain Integrity</div>
          <div style={{ display: "flex", gap: "2.5rem", alignItems: "flex-end", flexWrap: "wrap" }}>
            <Metric
              label="status"
              value={
                chain.valid ? (
                  <span className="pill ok">intact</span>
                ) : (
                  <span className="pill bad">broken @ {chain.first_broken_seq}</span>
                )
              }
            />
            <Metric label="entries checked" value={chain.entries_checked} />
            <Metric label="legacy skipped" value={chain.legacy_entries_skipped} />
          </div>
          <Drawer label="Raw chain report (JSON)">
            <pre className="code">{JSON.stringify(chain, null, 2)}</pre>
          </Drawer>
        </div>
      )}

      {/* Entry grid — outcome scannable at left; click a row to drill down. */}
      {entries.length > 0 && !loading && (
        <div className="card enter">
          <div className="card-title">Recent Verifications · {entries.length} · click a row to inspect</div>
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th>Outcome</th>
                  <th>Event</th>
                  <th>Session</th>
                  <th>Time</th>
                </tr>
              </thead>
              <tbody>
                {entries.map((e) => (
                  <tr
                    key={e.id}
                    className={`row-click ${selected?.id === e.id ? "row-selected" : ""}`}
                    tabIndex={0}
                    onClick={() => setSelected(e)}
                    onKeyDown={(ev) => ev.key === "Enter" && setSelected(e)}
                  >
                    <td>
                      <span className={`pill ${e.outcome === "success" ? "ok" : "bad"}`}>
                        {e.outcome}
                      </span>
                    </td>
                    <td>{e.event_type}</td>
                    <td>{e.session_id ? <Hash value={e.session_id} chars={6} /> : "—"}</td>
                    <td>{e.created_at}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Drill-down panel (D12) */}
      {selected && (
        <>
          <div className="panel-scrim" onClick={() => setSelected(null)} />
          <aside className="detail-panel glass" aria-label="Verification detail">
            <div className="detail-head">
              <div>
                <div className="card-title" style={{ marginBottom: "0.2rem" }}>
                  Verification · #{selected.id}
                </div>
                <span className={`pill ${selected.outcome === "success" ? "ok" : "bad"}`}>
                  {selected.outcome}
                </span>
              </div>
              <button
                type="button"
                className="ghost-btn"
                aria-label="Close detail panel"
                onClick={() => setSelected(null)}
              >
                <X size={16} />
              </button>
            </div>

            <div className="detail-section">
              <label>Timeline</label>
              <ul className="timeline">
                <li>
                  <span className="timeline-dot" />
                  <span>{selected.created_at}</span> — <code>{selected.event_type}</code>
                </li>
                <li>
                  <span className="timeline-dot" />
                  <span>chain position</span> — seq #{selected.id} (append-only)
                </li>
              </ul>
            </div>

            <div className="detail-section">
              <label>Session</label>
              {selected.session_id ? <Hash value={selected.session_id} chars={12} /> : "—"}
            </div>

            {typeof detail?.signature_b64 === "string" && (
              <div className="detail-section">
                <label>PQC signature ({String(detail.algo ?? "ML-DSA-65")})</label>
                <Hash value={detail.signature_b64} chars={16} />
              </div>
            )}

            {typeof detail?.chain_prev_sha256 === "string" && (
              <div className="detail-section">
                <label>Previous chain hash (SHA-256)</label>
                <Hash value={detail.chain_prev_sha256} chars={12} />
              </div>
            )}

            <div className="detail-section">
              <label>Full payload</label>
              <pre className="code">{JSON.stringify(selected, null, 2)}</pre>
            </div>
          </aside>
        </>
      )}
    </div>
  );
}
