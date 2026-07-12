// Route table (D1): every page has its own URL; back/forward work; everything
// except /login sits behind the auth gate (D2) inside the enterprise shell.

import { Navigate, Route, Routes } from "react-router-dom";
import { RequireAuth } from "./lib/auth";
import { Layout } from "./Layout";
import { ErrorBoundary } from "./components";
import { Login } from "./pages/Login";
import { Operations } from "./pages/Operations";
import { SystemOverview } from "./pages/SystemOverview";
import { Onboarding } from "./pages/Onboarding";
import { UiCustomizer } from "./pages/UiCustomizer";
import { AuditViewer } from "./pages/AuditViewer";
import { ApiKeys } from "./pages/ApiKeys";

export function App() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route
        element={
          <RequireAuth>
            <Layout />
          </RequireAuth>
        }
      >
        <Route path="/" element={<ErrorBoundary><Operations /></ErrorBoundary>} />
        <Route path="/architecture" element={<ErrorBoundary><SystemOverview /></ErrorBoundary>} />
        <Route path="/onboarding" element={<ErrorBoundary><Onboarding /></ErrorBoundary>} />
        <Route path="/customizer" element={<ErrorBoundary><UiCustomizer /></ErrorBoundary>} />
        <Route path="/audit" element={<ErrorBoundary><AuditViewer /></ErrorBoundary>} />
        <Route path="/keys" element={<ErrorBoundary><ApiKeys /></ErrorBoundary>} />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
