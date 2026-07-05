import { useState } from "react";
import { Onboarding } from "./pages/Onboarding";
import { UiCustomizer } from "./pages/UiCustomizer";
import { AuditViewer } from "./pages/AuditViewer";
import { ApiKeys } from "./pages/ApiKeys";

type Page = "onboarding" | "customizer" | "audit" | "keys";

const NAV: { id: Page; label: string }[] = [
  { id: "onboarding", label: "60-Minute Setup" },
  { id: "customizer", label: "UI Customizer" },
  { id: "audit", label: "Audit Log" },
  { id: "keys", label: "API Keys" },
];

export function App() {
  const [page, setPage] = useState<Page>("onboarding");

  return (
    <div className="app">
      <nav className="sidebar">
        <div className="sidebar-logo">◈ AJNA</div>
        <div className="sidebar-tag">Integration Console</div>
        {NAV.map((item) => (
          <button
            key={item.id}
            className={`nav-item ${page === item.id ? "active" : ""}`}
            onClick={() => setPage(item.id)}
          >
            {item.label}
          </button>
        ))}
      </nav>
      <main className="main">
        {page === "onboarding" && <Onboarding />}
        {page === "customizer" && <UiCustomizer />}
        {page === "audit" && <AuditViewer />}
        {page === "keys" && <ApiKeys />}
      </main>
    </div>
  );
}
