// Thin typed client for the Ajna verify backend. Base URL and API key are
// entered in the onboarding flow and kept in memory only (never persisted by
// the portal — the integrator owns key storage).

export interface AuditEntry {
  id: string;
  session_id: string | null;
  event_type: string;
  outcome: string;
  detail: unknown;
  created_at: string;
}

export interface AuditResponse {
  entries: AuditEntry[];
  total: number;
}

export interface ChainReport {
  valid: boolean;
  entries_checked: number;
  legacy_entries_skipped: number;
  first_broken_seq: number | null;
}

/** Backend base URL baked in at build time (set VITE_API_BASE in Vercel). */
export const DEFAULT_API_BASE: string =
  (import.meta.env.VITE_API_BASE as string | undefined) ?? "http://localhost:8080";

export class AjnaClient {
  constructor(private baseUrl: string, private apiKey: string) {}

  private async get<T>(path: string): Promise<T> {
    const res = await fetch(`${this.baseUrl.replace(/\/$/, "")}${path}`, {
      headers: { "X-Api-Key": this.apiKey },
    });
    if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
    return (await res.json()) as T;
  }

  health(): Promise<{ status: string } | unknown> {
    return this.get("/health");
  }

  auditLogs(limit = 50): Promise<AuditResponse> {
    return this.get(`/v1/audit?limit=${limit}`);
  }

  verifyChain(): Promise<ChainReport> {
    return this.get("/v1/audit/verify-chain");
  }
}
