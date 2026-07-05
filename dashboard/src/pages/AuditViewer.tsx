import { useState } from "react";
import { AjnaClient, DEFAULT_API_BASE, type AuditEntry, type ChainReport } from "../lib/api";
import { Drawer, Hash, Metric } from "../components";

export function AuditViewer() {
  const [baseUrl, setBaseUrl] = useState(DEFAULT_API_BASE);
  const [apiKey, setApiKey] = useState("");
  const [entries, setEntries] = useState<AuditEntry[]>([]);
  const [chain, setChain] = useState<ChainReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const client = () => new AjnaClient(baseUrl, apiKey);

  async function loadLogs() {
    setLoading(true);
    setError(null);
    try {
      const res = await client().auditLogs(50);
      setEntries(res.entries);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }

  async function checkChain() {
    setError(null);
    try {
      setChain(await client().verifyChain());
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div>
      <div className="hud-bar">
        <div>
          <h1 className="page-title">Audit Log</h1>
          <p className="page-subtitle">
            Tamper-evident SOC2 trail. Every verification is SHA-256 hash-chained.
          </p>
        </div>
        <span className="hud-coord">SEC//AUDIT-CHAIN</span>
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
            <input id="key" type="text" value={apiKey} onChange={(e) => setApiKey(e.target.value)} />
          </div>
        </div>
        <div style={{ display: "flex", gap: "0.6rem" }}>
          <button className="btn" onClick={loadLogs} disabled={loading}>
            {loading ? "Loading…" : "Load entries"}
          </button>
          <button className="btn secondary" onClick={checkChain}>
            Verify chain
          </button>
        </div>
        {error && <p className="error-text" style={{ marginTop: "0.6rem" }}>{error}</p>}
      </div>

      {/* Chain integrity summary — HUD metrics + collapsible detail. */}
      {chain && (
        <div className="card">
          <div className="card-title">Chain Integrity</div>
          <div style={{ display: "flex", gap: "2.5rem", alignItems: "flex-end" }}>
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

      {/* Entry grid — outcome scannable at left; hashes/JSON hidden in drawers. */}
      {entries.length > 0 && (
        <div className="card">
          <div className="card-title">Recent Verifications · {entries.length}</div>
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
                <tr key={e.id}>
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
          <Drawer label="Full entry payloads (JSON)">
            <pre className="code">{JSON.stringify(entries, null, 2)}</pre>
          </Drawer>
        </div>
      )}
    </div>
  );
}
