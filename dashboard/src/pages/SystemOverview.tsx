import { useState } from "react";
import { Drawer, Metric } from "../components";

interface Community {
  id: number;
  name: string;
  size: number;
  cohesion: number;
  lang: string;
  description: string;
}

interface Flow {
  name: string;
  criticality: number;
  nodeCount: number;
  description: string;
}

const COMMUNITIES: Community[] = [
  { id: 6, name: "src-face", size: 74, cohesion: 0.24, lang: "rust", description: "MediaPipe FaceMesh landmark geometry calculations (EAR, MAR, Yaw) and liveness FSM." },
  { id: 9, name: "src-mrz", size: 61, cohesion: 0.16, lang: "rust", description: "Identity document verification facade, Aadhaar Verhoeff checksums, and Passport ICAO MRZ parsing." },
  { id: 4, name: "rules-chain", size: 56, cohesion: 0.15, lang: "rust", description: "Country rules engine, NQM rules, and Postgres append-only SOC2 hash-chain triggers." },
  { id: 2, name: "src-ajna", size: 38, cohesion: 0.07, lang: "rust", description: "C FFI & JNI bindings, Ajna core session controller, and platform memory buffers." },
  { id: 1, name: "src-tool", size: 33, cohesion: 0.15, lang: "rust", description: "Stdio JSON-RPC 2.0 MCP server exposing 4 verification tools for agent workflows." },
  { id: 16, name: "sdk-native", size: 26, cohesion: 0.66, lang: "kotlin", description: "Android CameraX lifecycle bindings, MediaPipe JNI, and Android Sample ScanActivity." },
  { id: 11, name: "pages-patch", size: 21, cohesion: 0.16, lang: "tsx", description: "Vite Dashboard page routers, UI Customizer preview, and SOC2 Audit Chain viewer." },
  { id: 5, name: "src-report", size: 20, cohesion: 0.16, lang: "rust", description: "PostureReport evaluation, collecting client-side indicators and computing weighted scores." },
  { id: 15, name: "ajnaverifysample-scan", size: 20, cohesion: 0.60, lang: "swift", description: "iOS AVFoundation camera scanner, Swift SDK bindings, and SwiftUI Result/Scan Views." },
];

const FLOWS: Flow[] = [
  { name: "session_times_out_on_wall_clock_budget", criticality: 0.62, nodeCount: 3, description: "Ensures capture session fails automatically if liveness or document OCR exceeds budget limits." },
  { name: "attestation_signs_with_ml_dsa_and_verifies", criticality: 0.61, nodeCount: 2, description: "Signs verification results using on-device ML-DSA-65 private keys and validates them." },
  { name: "signed_result_verifies_and_tamper_is_rejected", criticality: 0.57, nodeCount: 5, description: "End-to-end cryptographic verification showing that any field modification voids the signature." },
  { name: "happy_path_clears_both_challenges", criticality: 0.52, nodeCount: 5, description: "Flow that tracks a client user successfully completing both liveness prompts (blink/smile/turn)." },
  { name: "ajna_session_push_result", criticality: 0.485, nodeCount: 2, description: "The FFI boundary push that moves raw camera frame analysis from Swift/Kotlin into Rust." },
];

export function SystemOverview() {
  const [hoveredNode, setHoveredNode] = useState<string | null>(null);

  const subsystems = [
    { id: "mobile", name: "Mobile Client (iOS/Android)", desc: "Captures frames, computes Vision landmarks, and queries Backend", color: "var(--accent)" },
    { id: "core", name: "Ajna Core (Rust/FFI)", desc: "Controls liveness state machine & parses doc bytes", color: "var(--ok)" },
    { id: "crypto", name: "Ajna Crypto (ML-DSA-65)", desc: "Signs validated fields at the edge using PQC signatures", color: "var(--accent)" },
    { id: "backend", name: "Axum Server (Backend)", desc: "Exposes HTTP endpoints & executes rules engines", color: "var(--warn)" },
    { id: "db", name: "Supabase DB (Postgres)", desc: "Holds tenant keys, seeds rules, & appends SOC2 chain", color: "var(--crit)" },
  ];

  return (
    <div className="system-overview">
      <div className="hud-bar">
        <div>
          <h1 className="page-title">SYSTEMS & ARCHITECTURE</h1>
          <p className="page-subtitle">Tactical overview of community boundaries, coupling, and execution flows</p>
        </div>
        <div className="hud-coord">SYS: AJNA-PLATFORM // STATIC SNAPSHOT · 2026-07</div>
      </div>

      {/* Metrics Row */}
      <div className="row" style={{ gridTemplateColumns: "repeat(4, 1fr)", marginBottom: "1.5rem" }}>
        <div className="card">
          <Metric label="Workspace Crates" value="8" />
        </div>
        <div className="card">
          <Metric label="Code Communities" value="16" />
        </div>
        <div className="card">
          <Metric label="Execution Flows" value="87" />
        </div>
        <div className="card">
          <Metric label="SOC2 Chain Status" value={<span style={{ color: "var(--ok)" }}>SECURE ✓</span>} />
        </div>
      </div>

      {/* Interactive Subsystem Coupling Diagram */}
      <div className="card" style={{ marginBottom: "1.5rem" }}>
        <h2 className="card-title">Subsystem Data Flows & Coupling</h2>
        <div style={{ display: "flex", gap: "2rem", flexDirection: "column" }}>
          <p className="text-2" style={{ fontSize: "0.82rem", lineHeight: "1.4" }}>
            Hover over a component below to trace its data relationships and directional coupling dependencies in the system.
          </p>

          <div style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))",
            gap: "1rem",
            margin: "1rem 0"
          }}>
            {subsystems.map((sub) => (
              <div
                key={sub.id}
                style={{
                  background: hoveredNode === sub.id ? "var(--elevated)" : "var(--surface-2)",
                  border: `1px solid ${hoveredNode === sub.id ? sub.color : "var(--border)"}`,
                  borderRadius: "4px",
                  padding: "1rem",
                  cursor: "pointer",
                  transition: "all 0.2s ease",
                  transform: hoveredNode === sub.id ? "scale(1.02)" : "none"
                }}
                onMouseEnter={() => setHoveredNode(sub.id)}
                onMouseLeave={() => setHoveredNode(null)}
              >
                <div style={{
                  fontFamily: "var(--mono)",
                  fontSize: "0.78rem",
                  fontWeight: "bold",
                  color: sub.color,
                  marginBottom: "0.5rem"
                }}>
                  {sub.name}
                </div>
                <p style={{ fontSize: "0.72rem", color: "var(--text-2)", lineHeight: "1.3" }}>
                  {sub.desc}
                </p>
              </div>
            ))}
          </div>

          {/* Dynamic Coupling Overlay */}
          <div style={{
            background: "var(--bg)",
            border: "1px solid var(--border)",
            padding: "0.85rem 1rem",
            borderRadius: "3px",
            fontFamily: "var(--mono)",
            fontSize: "0.72rem",
            minHeight: "70px"
          }}>
            <div style={{ color: "var(--text-3)", marginBottom: "0.25rem" }}>
              [ COUPLING RELATIONSHIP MATRIX ]
            </div>
            {hoveredNode === "mobile" && (
              <p style={{ color: "var(--accent)" }}>
                ➔ Mobile Client relies on: <b>Ajna Core</b> (dynamic linking of FFI/JNI) and calls <b>Axum Server</b> (via HTTPS REST API). Excludes backend libraries to run 100% locally.
              </p>
            )}
            {hoveredNode === "core" && (
              <p style={{ color: "var(--ok)" }}>
                ➔ Ajna Core couples: <b>Mobile client</b> (provides input frame pointers) and <b>Ajna Crypto</b> (triggers ML-DSA-65 signatures on validated structures). It is compiled as a static lib/AAR to prevent web leaks.
              </p>
            )}
            {hoveredNode === "crypto" && (
              <p style={{ color: "var(--accent)" }}>
                ➔ Ajna Crypto is coupled by: <b>Ajna Core</b> and <b>Axum Server</b>. Standardizes Dilithium-level keypairs across both edge signing and cloud verification.
              </p>
            )}
            {hoveredNode === "backend" && (
              <p style={{ color: "var(--warn)" }}>
                ➔ Axum Server interacts with: <b>Supabase DB</b> (executes rules queries & write chains) and receives payloads from <b>Mobile Client</b>. Implements token limits.
              </p>
            )}
            {hoveredNode === "db" && (
              <p style={{ color: "var(--crit)" }}>
                ➔ Supabase DB couples: <b>Axum Server</b>. Employs PL/pgSQL database triggers to automatically chain SHA-256 blocks of incoming verifications and alert on tamper.
              </p>
            )}
            {!hoveredNode && (
              <p style={{ color: "var(--text-3)" }}>
                No component hovered. Select a box above to inspect details.
              </p>
            )}
          </div>
        </div>
      </div>

      <div className="row" style={{ gridTemplateColumns: "1.2fr 0.8fr", gap: "1.5rem" }}>
        {/* Code Communities List */}
        <div className="card">
          <h2 className="card-title">Code Communities (Leiden Cluster Analysis)</h2>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem", marginTop: "0.5rem" }}>
            {COMMUNITIES.map((c) => (
              <Drawer
                key={c.id}
                label={`◈ ${c.name} — Size: ${c.size} nodes | Cohesion: ${c.cohesion} | ${c.lang.toUpperCase()}`}
              >
                <div style={{ padding: "0.35rem 0" }}>
                  <p style={{ fontSize: "0.78rem", color: "var(--text-2)", lineHeight: "1.4" }}>
                    {c.description}
                  </p>
                </div>
              </Drawer>
            ))}
          </div>
        </div>

        {/* Execution Flows */}
        <div className="card">
          <h2 className="card-title">Critical Execution Flows</h2>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.85rem", marginTop: "0.5rem" }}>
            {FLOWS.map((f) => (
              <div
                key={f.name}
                style={{
                  borderBottom: "1px solid var(--border)",
                  paddingBottom: "0.75rem"
                }}
              >
                <div style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "baseline",
                  marginBottom: "0.25rem"
                }}>
                  <span style={{
                    fontFamily: "var(--mono)",
                    fontSize: "0.72rem",
                    fontWeight: "bold",
                    color: "var(--accent)",
                    wordBreak: "break-all"
                  }}>
                    {f.name}
                  </span>
                  <span style={{
                    fontFamily: "var(--mono)",
                    fontSize: "0.68rem",
                    color: f.criticality > 0.6 ? "var(--crit)" : "var(--warn)",
                    marginLeft: "auto"
                  }}>
                    crit: {f.criticality}
                  </span>
                </div>
                <p style={{ fontSize: "0.70rem", color: "var(--text-2)", lineHeight: "1.3" }}>
                  {f.description}
                </p>
                <div style={{
                  fontFamily: "var(--mono)",
                  fontSize: "0.62rem",
                  color: "var(--text-3)",
                  marginTop: "0.25rem"
                }}>
                  Trace: {f.nodeCount} graph calls
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
