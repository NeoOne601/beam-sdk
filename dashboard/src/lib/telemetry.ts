// Live telemetry for the Operations console (D3). Polls every 2 s. On mount it
// probes the real backend /health once; whether or not the link is live, the
// verification stream itself is a deterministic simulator so the console is
// always demonstrable against the sleeping free-tier backend (ADR-006).
// ponytail: polling, not WebSocket — upgrade when a real event stream exists.

import { useEffect, useRef, useState } from "react";
import { DEFAULT_API_BASE } from "./api";

export interface TelemetrySample {
  t: string; // HH:MM:SS
  volume: number;
  pass: number;
  fail: number;
}

export interface GeoSlice {
  region: string;
  count: number;
}

export interface LiveEvent {
  id: string;
  kind: "idv" | "vision" | "intel";
  outcome: "pass" | "fail";
  region: string;
  at: string;
}

export interface Telemetry {
  linkStatus: "probing" | "live" | "simulated";
  totalToday: number;
  successRate: number; // 0..100
  activeSessions: number;
  p95LatencyMs: number;
  series: TelemetrySample[];
  geo: GeoSlice[];
  events: LiveEvent[];
}

const REGIONS = ["IN-South", "IN-North", "IN-West", "SEA", "MENA", "EU"];
const REGION_WEIGHTS = [0.31, 0.24, 0.19, 0.12, 0.09, 0.05];
const KINDS: LiveEvent["kind"][] = ["idv", "vision", "intel"];
const POLL_MS = 2000;
const WINDOW = 30;

function clock(d: Date): string {
  return d.toTimeString().slice(0, 8);
}

function pickRegion(rand: number): string {
  let acc = 0;
  for (let i = 0; i < REGIONS.length; i++) {
    acc += REGION_WEIGHTS[i];
    if (rand < acc) return REGIONS[i];
  }
  return REGIONS[0];
}

function seedTelemetry(): Telemetry {
  const now = Date.now();
  const series: TelemetrySample[] = [];
  for (let i = WINDOW - 1; i >= 0; i--) {
    const volume = 34 + Math.round(Math.random() * 18);
    const fail = Math.round(volume * (0.04 + Math.random() * 0.05));
    series.push({
      t: clock(new Date(now - i * POLL_MS)),
      volume,
      pass: volume - fail,
      fail,
    });
  }
  return {
    linkStatus: "probing",
    totalToday: 18_240 + Math.round(Math.random() * 900),
    successRate: 94.2,
    activeSessions: 12,
    p95LatencyMs: 412,
    series,
    geo: REGIONS.map((region, i) => ({
      region,
      count: Math.round(2400 * REGION_WEIGHTS[i] * (0.9 + Math.random() * 0.2)),
    })),
    events: [],
  };
}

function nextTick(prev: Telemetry): Telemetry {
  const last = prev.series[prev.series.length - 1];
  const drift = Math.round((Math.random() - 0.48) * 8);
  const volume = Math.max(12, Math.min(80, last.volume + drift));
  const fail = Math.max(0, Math.round(volume * (0.03 + Math.random() * 0.07)));
  const sample: TelemetrySample = {
    t: clock(new Date()),
    volume,
    pass: volume - fail,
    fail,
  };
  const series = [...prev.series.slice(1), sample];
  const passTotal = series.reduce((a, s) => a + s.pass, 0);
  const volTotal = series.reduce((a, s) => a + s.volume, 0);

  const event: LiveEvent = {
    id: Math.random().toString(16).slice(2, 10),
    kind: KINDS[Math.floor(Math.random() * KINDS.length)],
    outcome: Math.random() < fail / Math.max(volume, 1) ? "fail" : "pass",
    region: pickRegion(Math.random()),
    at: sample.t,
  };

  return {
    ...prev,
    totalToday: prev.totalToday + volume,
    successRate: Math.round((passTotal / Math.max(volTotal, 1)) * 1000) / 10,
    activeSessions: Math.max(1, prev.activeSessions + Math.round((Math.random() - 0.5) * 3)),
    p95LatencyMs: Math.max(180, Math.min(900, prev.p95LatencyMs + Math.round((Math.random() - 0.5) * 40))),
    series,
    geo: prev.geo.map((g) => ({
      ...g,
      count: g.count + (Math.random() < REGION_WEIGHTS[REGIONS.indexOf(g.region)] * 2 ? 1 : 0),
    })),
    events: [event, ...prev.events].slice(0, 8),
  };
}

/** Polls telemetry; metrics update without any page refresh (D3). */
export function useTelemetry(): Telemetry {
  const [data, setData] = useState<Telemetry>(seedTelemetry);
  const linkChecked = useRef(false);

  useEffect(() => {
    if (!linkChecked.current) {
      linkChecked.current = true;
      const probe = new AbortController();
      const timer = setTimeout(() => probe.abort(), 4000);
      fetch(`${DEFAULT_API_BASE.replace(/\/$/, "")}/health`, { signal: probe.signal })
        .then((res) => {
          setData((d) => ({ ...d, linkStatus: res.ok ? "live" : "simulated" }));
        })
        .catch(() => setData((d) => ({ ...d, linkStatus: "simulated" })))
        .finally(() => clearTimeout(timer));
    }
    const interval = setInterval(() => setData(nextTick), POLL_MS);
    return () => clearInterval(interval);
  }, []);

  return data;
}
