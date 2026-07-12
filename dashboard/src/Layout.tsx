// Enterprise shell (D5, D6): collapsible icon+label sidebar, top command bar
// with breadcrumbs / org / avatar / UTC clock, mobile overlay drawer, and the
// global Cmd+K palette. Pages render into <Outlet/>.

import { useEffect, useState } from "react";
import { NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import {
  Activity,
  FileSearch,
  KeyRound,
  LogOut,
  Menu,
  Network,
  Palette,
  PanelLeftClose,
  PanelLeftOpen,
  Rocket,
  Search,
  X,
  type LucideIcon,
} from "lucide-react";
import { useAuth } from "./lib/auth";
import { CommandPalette } from "./CommandPalette";

export interface NavEntry {
  path: string;
  label: string;
  crumb: string;
  icon: LucideIcon;
}

export const NAV: NavEntry[] = [
  { path: "/", label: "Operations", crumb: "Operations Overview", icon: Activity },
  { path: "/architecture", label: "Architecture", crumb: "Systems & Architecture", icon: Network },
  { path: "/onboarding", label: "60-Min Setup", crumb: "60-Minute Integration", icon: Rocket },
  { path: "/customizer", label: "UI Customizer", crumb: "Capture UI Customizer", icon: Palette },
  { path: "/audit", label: "Audit Log", crumb: "SOC2 Audit Chain", icon: FileSearch },
  { path: "/keys", label: "API Keys", crumb: "API Key Management", icon: KeyRound },
];

function useUtcClock(): string {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = setInterval(() => setNow(new Date()), 1000);
    return () => clearInterval(id);
  }, []);
  return `${now.toISOString().slice(11, 19)}Z`;
}

export function Layout() {
  const { session, logout } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const clock = useUtcClock();
  const [collapsed, setCollapsed] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);

  // Cmd+K / Ctrl+K / "/" opens the command palette anywhere (D7).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const inField =
        e.target instanceof HTMLElement &&
        ["INPUT", "TEXTAREA", "SELECT"].includes(e.target.tagName);
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPaletteOpen((v) => !v);
      } else if (e.key === "/" && !inField) {
        e.preventDefault();
        setPaletteOpen(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Close the mobile drawer on navigation.
  useEffect(() => setMobileOpen(false), [location.pathname]);

  // Close the drawer if the viewport grows past the mobile breakpoint —
  // a lingering open drawer would otherwise dim and block the desktop UI.
  useEffect(() => {
    const onResize = () => window.innerWidth > 768 && setMobileOpen(false);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  const active = NAV.find(
    (n) => n.path === location.pathname || (n.path !== "/" && location.pathname.startsWith(n.path)),
  );
  const initials = (session?.email ?? "??").slice(0, 2).toUpperCase();

  return (
    <div className={`app ${collapsed ? "rail" : ""}`}>
      {mobileOpen && <div className="sidebar-scrim" onClick={() => setMobileOpen(false)} />}
      <nav className={`sidebar ${mobileOpen ? "open" : ""}`} aria-label="Primary">
        <div className="sidebar-head">
          <div>
            <div className="sidebar-logo">◈ {collapsed ? "" : "AJNA"}</div>
            {!collapsed && <div className="sidebar-tag">Integration Console</div>}
          </div>
          <button
            type="button"
            className="ghost-btn sidebar-close"
            aria-label="Close menu"
            onClick={() => setMobileOpen(false)}
          >
            <X size={16} />
          </button>
        </div>
        {NAV.map(({ path, label, icon: Icon }) => (
          <NavLink
            key={path}
            to={path}
            end={path === "/"}
            className={({ isActive }) => `nav-item ${isActive ? "active" : ""}`}
            title={label}
          >
            <Icon size={15} strokeWidth={1.75} aria-hidden="true" />
            <span className="nav-label">{label}</span>
          </NavLink>
        ))}
        <div className="sidebar-foot">
          <button
            type="button"
            className="ghost-btn collapse-toggle"
            aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
            onClick={() => setCollapsed((v) => !v)}
          >
            {collapsed ? <PanelLeftOpen size={15} /> : <PanelLeftClose size={15} />}
            <span className="nav-label">Collapse</span>
          </button>
        </div>
      </nav>

      <div className="frame">
        <header className="topbar">
          <button
            type="button"
            className="ghost-btn hamburger"
            aria-label="Open menu"
            onClick={() => setMobileOpen(true)}
          >
            <Menu size={17} />
          </button>
          <nav className="breadcrumbs" aria-label="Breadcrumb">
            <span className="crumb-root">CONSOLE</span>
            <span className="crumb-sep">/</span>
            <span className="crumb-here">{active?.crumb ?? "Operations Overview"}</span>
          </nav>
          <div className="topbar-right">
            <button
              type="button"
              className="ghost-btn palette-hint"
              onClick={() => setPaletteOpen(true)}
            >
              <Search size={13} />
              <span>Search</span>
              <kbd>⌘K</kbd>
            </button>
            <span className="topbar-clock">{clock}</span>
            <span className="topbar-org">{session?.org ?? ""}</span>
            <span className="avatar" title={session?.email ?? ""}>{initials}</span>
            <button
              type="button"
              className="ghost-btn"
              aria-label="Sign out"
              title="Sign out"
              onClick={() => {
                logout();
                navigate("/login");
              }}
            >
              <LogOut size={15} />
            </button>
          </div>
        </header>
        <main className="main">
          <Outlet />
        </main>
      </div>

      {paletteOpen && <CommandPalette onClose={() => setPaletteOpen(false)} />}
    </div>
  );
}
