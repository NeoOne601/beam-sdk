import { useState } from "react";
import { AjnaClient, type AuditEntry, type ChainReport } from "../lib/api";

export function AuditViewer() {
  const [baseUrl, setBaseUrl] = useState("http://localhost:8080");
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
      <h1 className="page-title">Audit Log</h1>
      <p className="page-subtitle">
        The tamper-evident SOC2 trail. Every verification is hash-chained; use "Verify chain"
        to prove no entry was altered or removed.
      </p>

      <div className="card">
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
        <div style={{ display: "flex", gap: "0.75rem" }}>
          <button className="btn" onClick={loadLogs} disabled={loading}>
            {loading ? "Loading…" : "Load recent entries"}
          </button>
          <button className="btn secondary" onClick={checkChain}>
            Verify chain
          </button>
        </div>
        {error && <p className="error-text" style={{ marginTop: "0.75rem" }}>{error}</p>}
        {chain && (
          <p style={{ marginTop: "0.75rem" }}>
            {chain.valid ? (
              <span className="pill ok">chain intact</span>
            ) : (
              <span className="pill bad">broken at seq {chain.first_broken_seq}</span>
            )}{" "}
            <span className="page-subtitle">
              {chain.entries_checked} checked, {chain.legacy_entries_skipped} legacy skipped
            </span>
          </p>
        )}
      </div>

      {entries.length > 0 && (
        <div className="card">
          <table>
            <thead>
              <tr>
                <th>Event</th>
                <th>Outcome</th>
                <th>Session</th>
                <th>Time</th>
              </tr>
            </thead>
            <tbody>
              {entries.map((e) => (
                <tr key={e.id}>
                  <td>{e.event_type}</td>
                  <td>
                    <span className={`pill ${e.outcome === "success" ? "ok" : "bad"}`}>
                      {e.outcome}
                    </span>
                  </td>
                  <td>{e.session_id ?? "—"}</td>
                  <td>{e.created_at}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
