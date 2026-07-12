// Authentication gate (D2). Demo credentials are printed on the card —
// this is the integration-portal demo tenant (ADR-007), not production auth.

import { useState, type FormEvent } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { ShieldCheck } from "lucide-react";
import { DEMO_EMAIL, DEMO_PASSWORD, useAuth } from "../lib/auth";

export function Login() {
  const { login } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);

  function submit(e: FormEvent) {
    e.preventDefault();
    const failure = login(email, password);
    if (failure) {
      setError(failure);
      return;
    }
    const from = (location.state as { from?: string } | null)?.from;
    navigate(from && from !== "/login" ? from : "/", { replace: true });
  }

  return (
    <div className="login-screen">
      <form className="card login-card enter" onSubmit={submit}>
        <div className="login-logo">
          ◈ AJNA
          <span className="sidebar-tag" style={{ padding: 0 }}>Integration Console</span>
        </div>

        <label htmlFor="email">Email</label>
        <input
          id="email"
          type="email"
          autoComplete="username"
          value={email}
          placeholder={DEMO_EMAIL}
          onChange={(e) => setEmail(e.target.value)}
        />

        <label htmlFor="password">Password</label>
        <input
          id="password"
          type="password"
          autoComplete="current-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
        />

        {error && <p className="error-text">{error}</p>}

        <button type="submit" className="btn login-btn" disabled={!email || !password}>
          Authenticate
        </button>

        <div className="login-demo">
          <ShieldCheck size={13} aria-hidden="true" />
          <span>
            Demo tenant — <code>{DEMO_EMAIL}</code> / <code>{DEMO_PASSWORD}</code>
          </span>
        </div>
      </form>
      <div className="hud-coord login-coord">SEC//PORTAL-GATE · SESSION TOKEN: HS256 (DEMO)</div>
    </div>
  );
}
