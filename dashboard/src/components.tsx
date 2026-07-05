// Shared tactical-HUD primitives: progressive-disclosure drawers and a
// monospaced hash chip that hides the full value until expanded — the
// "hide cryptographic detail behind a drawer" pattern used across screens.

import { useState, type ReactNode } from "react";

/** Collapsible drawer (native <details>) for hashes, JSON, and dense detail. */
export function Drawer({
  label,
  children,
  open = false,
}: {
  label: string;
  children: ReactNode;
  open?: boolean;
}) {
  return (
    <details className="drawer" open={open}>
      <summary>{label}</summary>
      <div className="drawer-body">{children}</div>
    </details>
  );
}

/** A cryptographic hash: shows head…tail, click to toggle the full value. */
export function Hash({ value, chars = 10 }: { value: string; chars?: number }) {
  const [full, setFull] = useState(false);
  if (!value) return <span className="hash">∅</span>;
  const truncatable = value.length > chars * 2 + 1;
  const shown = full || !truncatable ? value : `${value.slice(0, chars)}…${value.slice(-chars)}`;
  return (
    <span
      className={`hash ${truncatable ? "hash-trunc" : ""}`}
      title={truncatable ? "click to expand" : value}
      onClick={() => truncatable && setFull((f) => !f)}
    >
      {shown}
    </span>
  );
}

/** A labeled HUD metric (big monospace number over a micro-label). */
export function Metric({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div>
      <div className="metric">{value}</div>
      <div className="metric-label">{label}</div>
    </div>
  );
}
