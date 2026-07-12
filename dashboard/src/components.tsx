// Shared tactical-HUD primitives: progressive-disclosure drawers, hash chips,
// count-up metrics, skeletons, error boundary, copy button, and the
// glassmorphism confirm dialog. One design language across every screen.

import {
  Component,
  useEffect,
  useRef,
  useState,
  type ErrorInfo,
  type ReactNode,
} from "react";
import { Check, Copy } from "lucide-react";
import { useToast } from "./lib/toast";

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

/** Animates a numeric value toward its target — metric count-up (§15 motion). */
function useCountUp(target: number): number {
  const [shown, setShown] = useState(target);
  const raf = useRef(0);
  useEffect(() => {
    const from = shown;
    const delta = target - from;
    if (delta === 0) return;
    const start = performance.now();
    const DURATION = 500;
    const step = (now: number) => {
      const p = Math.min(1, (now - start) / DURATION);
      setShown(Math.round(from + delta * (1 - (1 - p) * (1 - p))));
      if (p < 1) raf.current = requestAnimationFrame(step);
    };
    raf.current = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf.current);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [target]);
  return shown;
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

/** Metric variant that counts up when its numeric value changes. */
export function LiveMetric({
  label,
  value,
  suffix = "",
  decimals = 0,
}: {
  label: string;
  value: number;
  suffix?: string;
  decimals?: number;
}) {
  // Count in scaled-integer space so fractional targets (94.2%) don't round away.
  const scale = 10 ** decimals;
  const shown = useCountUp(Math.round(value * scale)) / scale;
  return (
    <Metric
      label={label}
      value={`${shown.toLocaleString(undefined, {
        minimumFractionDigits: decimals,
        maximumFractionDigits: decimals,
      })}${suffix}`}
    />
  );
}

/** Skeleton shimmer block for loading states (D8). */
export function Skeleton({ lines = 3 }: { lines?: number }) {
  return (
    <div aria-hidden="true">
      {Array.from({ length: lines }, (_, i) => (
        <div key={i} className="skeleton" style={{ width: `${100 - i * 12}%` }} />
      ))}
    </div>
  );
}

/** Copy-to-clipboard with visual + toast confirmation (D11). */
export function CopyButton({ text, label = "Copy" }: { text: string; label?: string }) {
  const push = useToast();
  const [copied, setCopied] = useState(false);
  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      push("Copied to clipboard", "ok");
      setTimeout(() => setCopied(false), 1600);
    } catch {
      push("Clipboard unavailable", "warn");
    }
  }
  return (
    <button type="button" className="btn secondary btn-icon" onClick={copy}>
      {copied ? <Check size={13} /> : <Copy size={13} />}
      {copied ? "Copied" : label}
    </button>
  );
}

/** Glassmorphism confirmation dialog. Escape closes; focus lands inside (D10). */
export function ConfirmDialog({
  title,
  body,
  confirmLabel,
  danger = false,
  onConfirm,
  onClose,
}: {
  title: string;
  body: ReactNode;
  confirmLabel: string;
  danger?: boolean;
  onConfirm: () => void;
  onClose: () => void;
}) {
  const confirmRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    confirmRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="modal-scrim" onClick={onClose}>
      <div
        className="modal glass"
        role="dialog"
        aria-modal="true"
        aria-label={title}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="card-title">{title}</div>
        <div className="modal-body">{body}</div>
        <div className="modal-actions">
          <button type="button" className="btn secondary" onClick={onClose}>
            Cancel
          </button>
          <button
            ref={confirmRef}
            type="button"
            className={`btn ${danger ? "danger" : ""}`}
            onClick={() => {
              onConfirm();
              onClose();
            }}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}

/** Error boundary with a retry button — no raw stack traces to users (D8). */
export class ErrorBoundary extends Component<
  { children: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[ajna-portal] view crashed", error, info.componentStack);
  }

  render() {
    if (!this.state.failed) return this.props.children;
    return (
      <div className="card error-boundary">
        <div className="card-title">Signal lost</div>
        <p className="page-subtitle">
          This view hit an unexpected error. Your session and data are intact.
        </p>
        <button
          type="button"
          className="btn"
          onClick={() => this.setState({ failed: false })}
        >
          Retry
        </button>
      </div>
    );
  }
}
