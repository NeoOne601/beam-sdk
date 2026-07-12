// Cmd+K command palette (D7): fuzzy-ish filter over navigation + actions,
// arrow-key selection, Enter to run, Escape to close. Glassmorphism overlay.

import { useEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { useNavigate } from "react-router-dom";
import { CornerDownLeft, LogOut } from "lucide-react";
import { NAV } from "./Layout";
import { useAuth } from "./lib/auth";

interface Command {
  id: string;
  label: string;
  hint: string;
  run: () => void;
}

export function CommandPalette({ onClose }: { onClose: () => void }) {
  const navigate = useNavigate();
  const { logout } = useAuth();
  const [query, setQuery] = useState("");
  const [cursor, setCursor] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands = useMemo<Command[]>(
    () => [
      ...NAV.map((n) => ({
        id: n.path,
        label: n.crumb,
        hint: "Navigate",
        run: () => navigate(n.path),
      })),
      {
        id: "logout",
        label: "Sign out of the console",
        hint: "Session",
        run: () => {
          logout();
          navigate("/login");
        },
      },
    ],
    [navigate, logout],
  );

  const matches = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return commands;
    return commands.filter(
      (c) => c.label.toLowerCase().includes(q) || c.id.toLowerCase().includes(q),
    );
  }, [commands, query]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => setCursor(0), [query]);

  function onKeyDown(e: ReactKeyboardEvent) {
    if (e.key === "Escape") onClose();
    else if (e.key === "ArrowDown") {
      e.preventDefault();
      setCursor((c) => Math.min(c + 1, matches.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setCursor((c) => Math.max(c - 1, 0));
    } else if (e.key === "Enter" && matches[cursor]) {
      matches[cursor].run();
      onClose();
    }
  }

  return (
    <div className="modal-scrim palette-scrim" onClick={onClose}>
      <div
        className="palette glass"
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        <input
          ref={inputRef}
          type="text"
          className="palette-input"
          placeholder="Jump to… (type to filter)"
          value={query}
          aria-label="Search commands"
          onChange={(e) => setQuery(e.target.value)}
        />
        <ul className="palette-list" role="listbox">
          {matches.length === 0 && <li className="palette-empty">No matches</li>}
          {matches.map((c, i) => (
            <li
              key={c.id}
              role="option"
              aria-selected={i === cursor}
              className={`palette-item ${i === cursor ? "active" : ""}`}
              onMouseEnter={() => setCursor(i)}
              onClick={() => {
                c.run();
                onClose();
              }}
            >
              {c.id === "logout" ? <LogOut size={13} /> : <CornerDownLeft size={13} />}
              <span>{c.label}</span>
              <span className="palette-hint-tag">{c.hint}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
