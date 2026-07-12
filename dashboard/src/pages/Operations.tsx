// Operations overview — the landing page (D3 live telemetry, D4 charts).
// Volume time-series, pass/fail donut, geographic distribution, live event
// feed. Everything updates on the 2 s telemetry tick without a page refresh.

import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { LiveMetric, Metric } from "../components";
import { useTelemetry } from "../lib/telemetry";

const CHART_TEXT = { fontFamily: "JetBrains Mono, monospace", fontSize: 10, fill: "#4a5b6b" };
const TOOLTIP_STYLE = {
  background: "rgba(13, 19, 26, 0.92)",
  border: "1px solid #294152",
  borderRadius: 3,
  fontFamily: "JetBrains Mono, monospace",
  fontSize: 11,
};

export function Operations() {
  const t = useTelemetry();
  const passCount = t.series.reduce((a, s) => a + s.pass, 0);
  const failCount = t.series.reduce((a, s) => a + s.fail, 0);
  const donut = [
    { name: "pass", value: passCount },
    { name: "fail", value: failCount },
  ];

  return (
    <div>
      <div className="hud-bar">
        <div>
          <h1 className="page-title">Operations Overview</h1>
          <p className="page-subtitle">Live verification telemetry across all three pillars.</p>
        </div>
        <div className="hud-right">
          <span className={`pill ${t.linkStatus === "live" ? "ok" : "warn"}`}>
            {t.linkStatus === "live"
              ? "backend link live"
              : t.linkStatus === "probing"
                ? "probing link…"
                : "simulated feed"}
          </span>
          <span className="hud-coord">OPS//TELEMETRY · POLL 2S</span>
        </div>
      </div>

      <div className="metric-grid">
        <div className="card enter d-0"><LiveMetric label="Verifications today" value={t.totalToday} /></div>
        <div className="card enter d-1"><LiveMetric label="Success rate" value={t.successRate} suffix="%" decimals={1} /></div>
        <div className="card enter d-2"><LiveMetric label="Active sessions" value={t.activeSessions} /></div>
        <div className="card enter d-3"><LiveMetric label="p95 latency" value={t.p95LatencyMs} suffix=" ms" /></div>
      </div>

      <div className="chart-grid">
        <div className="card enter d-1">
          <div className="card-title">Verification volume · rolling window</div>
          <ResponsiveContainer width="100%" height={220}>
            <AreaChart data={t.series} margin={{ top: 4, right: 8, left: -18, bottom: 0 }}>
              <defs>
                <linearGradient id="vol" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#38bdf8" stopOpacity={0.35} />
                  <stop offset="100%" stopColor="#38bdf8" stopOpacity={0.02} />
                </linearGradient>
              </defs>
              <CartesianGrid stroke="#1c2833" strokeDasharray="2 4" vertical={false} />
              <XAxis dataKey="t" tick={CHART_TEXT} tickLine={false} axisLine={{ stroke: "#1c2833" }} interval={6} />
              <YAxis tick={CHART_TEXT} tickLine={false} axisLine={false} width={46} />
              <Tooltip contentStyle={TOOLTIP_STYLE} labelStyle={{ color: "#8da0b4" }} />
              <Area type="monotone" dataKey="volume" stroke="#38bdf8" strokeWidth={1.5} fill="url(#vol)" isAnimationActive={false} />
              <Area type="monotone" dataKey="fail" stroke="#f87171" strokeWidth={1} fill="transparent" isAnimationActive={false} />
            </AreaChart>
          </ResponsiveContainer>
        </div>

        <div className="card enter d-2">
          <div className="card-title">Pass / fail ratio</div>
          <div className="donut-wrap">
            <ResponsiveContainer width="100%" height={180}>
              <PieChart>
                <Pie
                  data={donut}
                  dataKey="value"
                  innerRadius={54}
                  outerRadius={76}
                  strokeWidth={0}
                  isAnimationActive={false}
                >
                  <Cell fill="#34d399" />
                  <Cell fill="#f87171" />
                </Pie>
                <Tooltip contentStyle={TOOLTIP_STYLE} />
              </PieChart>
            </ResponsiveContainer>
            <div className="donut-center">
              <Metric label="pass rate" value={`${t.successRate}%`} />
            </div>
          </div>
          <div className="legend">
            <span className="legend-dot" style={{ background: "#34d399" }} /> pass · {passCount.toLocaleString()}
            <span className="legend-dot" style={{ background: "#f87171", marginLeft: "1rem" }} /> fail · {failCount.toLocaleString()}
          </div>
        </div>
      </div>

      <div className="chart-grid">
        <div className="card enter d-2">
          <div className="card-title">Geographic distribution · 24h</div>
          <ResponsiveContainer width="100%" height={190}>
            <BarChart data={t.geo} layout="vertical" margin={{ top: 0, right: 12, left: 4, bottom: 0 }}>
              <CartesianGrid stroke="#1c2833" strokeDasharray="2 4" horizontal={false} />
              <XAxis type="number" tick={CHART_TEXT} tickLine={false} axisLine={false} />
              <YAxis type="category" dataKey="region" tick={CHART_TEXT} tickLine={false} axisLine={false} width={70} />
              <Tooltip contentStyle={TOOLTIP_STYLE} cursor={{ fill: "rgba(56,189,248,0.06)" }} />
              <Bar dataKey="count" fill="#1e6a8c" radius={[0, 2, 2, 0]} isAnimationActive={false} />
            </BarChart>
          </ResponsiveContainer>
        </div>

        <div className="card enter d-3">
          <div className="card-title">Live event feed</div>
          <ul className="event-feed">
            {t.events.length === 0 && <li className="palette-empty">Awaiting first tick…</li>}
            {t.events.map((e) => (
              <li key={e.id} className="event-row">
                <span className={`pill ${e.outcome === "pass" ? "ok" : "bad"}`}>{e.outcome}</span>
                <span className="event-kind">{e.kind.toUpperCase()}</span>
                <span className="event-region">{e.region}</span>
                <span className="event-time">{e.at}</span>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </div>
  );
}
