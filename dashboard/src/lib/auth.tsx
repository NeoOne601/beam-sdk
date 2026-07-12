// Portal auth gate (ADR-007): demo credentials validated client-side, a
// clearly-labeled demo JWT kept in sessionStorage. Real portal-user identity
// is a Phase 2 backend feature — this gate exists so the console is never
// reachable unauthenticated (D2) and the session shape matches production.

import {
  createContext,
  useContext,
  useEffect,
  useReducer,
  type ReactNode,
} from "react";
import { Navigate, useLocation } from "react-router-dom";

export const DEMO_EMAIL = "demo@ajna.io";
export const DEMO_PASSWORD = "ajna-demo";
const STORAGE_KEY = "ajna.portal.session";
const SESSION_HOURS = 8;

export interface Session {
  token: string;
  email: string;
  org: string;
  expiresAt: number; // epoch ms
}

type AuthState = { session: Session | null };
type AuthAction = { type: "login"; session: Session } | { type: "logout" };

function b64url(value: string): string {
  return btoa(value).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

// Demo JWT: structurally valid header.payload.signature so integrators see
// the real session shape; the signature is a literal demo marker, not a MAC.
function mintDemoJwt(email: string, expiresAt: number): string {
  const header = b64url(JSON.stringify({ alg: "HS256", typ: "JWT" }));
  const payload = b64url(
    JSON.stringify({
      sub: email,
      org: "acme-fintech",
      plan: "demo",
      exp: Math.floor(expiresAt / 1000),
    }),
  );
  return `${header}.${payload}.demo-signature-not-for-production`;
}

function loadSession(): Session | null {
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const session = JSON.parse(raw) as Session;
    return session.expiresAt > Date.now() ? session : null;
  } catch {
    return null;
  }
}

function reducer(_state: AuthState, action: AuthAction): AuthState {
  switch (action.type) {
    case "login":
      return { session: action.session };
    case "logout":
      return { session: null };
  }
}

interface AuthApi {
  session: Session | null;
  login: (email: string, password: string) => string | null; // error message or null
  logout: () => void;
}

const AuthContext = createContext<AuthApi | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, { session: loadSession() });

  useEffect(() => {
    if (state.session) {
      sessionStorage.setItem(STORAGE_KEY, JSON.stringify(state.session));
    } else {
      sessionStorage.removeItem(STORAGE_KEY);
    }
  }, [state.session]);

  const login = (email: string, password: string): string | null => {
    if (email.trim().toLowerCase() !== DEMO_EMAIL || password !== DEMO_PASSWORD) {
      return "Invalid credentials. Use the demo credentials shown below.";
    }
    const expiresAt = Date.now() + SESSION_HOURS * 3600_000;
    dispatch({
      type: "login",
      session: {
        token: mintDemoJwt(email, expiresAt),
        email: DEMO_EMAIL,
        org: "ACME FINTECH",
        expiresAt,
      },
    });
    return null;
  };

  const logout = () => dispatch({ type: "logout" });

  return (
    <AuthContext.Provider value={{ session: state.session, login, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthApi {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth outside AuthProvider");
  return ctx;
}

/** Route guard: unauthenticated users see nothing but /login (D2). */
export function RequireAuth({ children }: { children: ReactNode }) {
  const { session } = useAuth();
  const location = useLocation();
  if (!session || session.expiresAt <= Date.now()) {
    return <Navigate to="/login" replace state={{ from: location.pathname }} />;
  }
  return <>{children}</>;
}
