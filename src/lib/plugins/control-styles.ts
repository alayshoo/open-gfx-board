/**
 * Plugin Control UI – Shared Touch-First Button & Layout System
 *
 * These styles are injected into the document by PluginHost before any plugin
 * component is mounted.  Plugin authors can rely on all `ctrl-*` classes being
 * available without importing anything themselves.
 *
 * Class reference
 * ───────────────
 * Buttons
 *   .ctrl-btn          Base touch button (52 px min-height, 1 rem text)
 *   .ctrl-btn-lg       Large variant     (60 px min-height, 1.1 rem text)
 *   .ctrl-btn-sq       Square icon/±     (56 px min, no side padding, 1.5 rem text)
 *   .ctrl-btn-full     Stretches to full container width
 *
 * Button colour variants (combine with .ctrl-btn or .ctrl-btn-lg)
 *   .ctrl-btn-goal     Green  – positive events
 *   .ctrl-btn-warn     Yellow – caution / warning events
 *   .ctrl-btn-danger   Red    – negative / destructive events
 *   .ctrl-btn-accent   Cyan   – informational / secondary actions
 *   .ctrl-btn-active   Green glow – toggled-on / running state
 *
 * Layout
 *   .ctrl-row          Horizontal flex row; each direct child gets flex:1
 *   .ctrl-grid-2       2-column equal grid
 *   .ctrl-grid-3       3-column equal grid
 *
 * Form controls
 *   .ctrl-input        Touch-sized text/number input  (52 px min-height)
 *   .ctrl-input-num    Compact centred number input   (72 px wide)
 *   .ctrl-select       Touch-sized <select>           (52 px min-height)
 */

export const CONTROL_STYLES = /* css */ `
/* ── Base button ─────────────────────────────────────────────────────── */
.ctrl-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 52px;
  padding: 10px 20px;
  background: var(--surface-3, #3f3f46);
  border: 1px solid var(--border-2, #52525b);
  border-radius: 10px;
  color: var(--text-1, #e4e4e7);
  cursor: pointer;
  font-size: 1rem;
  font-weight: 600;
  font-family: inherit;
  line-height: 1;
  white-space: nowrap;
  transition: background 0.1s, transform 0.08s;
  user-select: none;
  -webkit-tap-highlight-color: transparent;
  box-sizing: border-box;
}
.ctrl-btn:hover  { background: var(--surface-4, #52525b); }
.ctrl-btn:active { transform: scale(0.96); }

/* ── Size modifiers ──────────────────────────────────────────────────── */
.ctrl-btn-lg {
  min-height: 60px;
  font-size: 1.05rem;
  border-radius: 12px;
  padding: 12px 22px;
}

/* Square button – for ± score controls, icon-only actions, etc.
   Intended to be used inside a .ctrl-row so flex:1 sets the width. */
.ctrl-btn-sq {
  min-height: 56px;
  padding: 0;
  font-size: 1.5rem;
  border-radius: 10px;
}

/* Stretch to the full width of its container */
.ctrl-btn-full {
  width: 100%;
}

/* ── Colour variants ─────────────────────────────────────────────────── */
.ctrl-btn-goal   { background: rgba(34,197,94,0.14);  border-color: rgba(34,197,94,0.35);  color: #22c55e; }
.ctrl-btn-goal:hover   { background: rgba(34,197,94,0.26); }

.ctrl-btn-warn   { background: rgba(234,179,8,0.14);  border-color: rgba(234,179,8,0.35);  color: #eab308; }
.ctrl-btn-warn:hover   { background: rgba(234,179,8,0.26); }

.ctrl-btn-danger { background: rgba(239,68,68,0.14);  border-color: rgba(239,68,68,0.35);  color: #ef4444; }
.ctrl-btn-danger:hover { background: rgba(239,68,68,0.26); }

.ctrl-btn-accent { background: rgba(56,189,248,0.14); border-color: rgba(56,189,248,0.35); color: #38bdf8; }
.ctrl-btn-accent:hover { background: rgba(56,189,248,0.26); }

/* Toggled-on / running state – applied dynamically via JS */
.ctrl-btn-active {
  background: rgba(34,197,94,0.14) !important;
  border-color: rgba(34,197,94,0.4) !important;
  color: #22c55e !important;
}

/* ── Layout utilities ────────────────────────────────────────────────── */

/* Horizontal row – every direct child expands equally */
.ctrl-row {
  display: flex;
  gap: 8px;
  width: 100%;
}
.ctrl-row > * { flex: 1; min-width: 0; }

.ctrl-grid-2 {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  width: 100%;
}

.ctrl-grid-3 {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 8px;
  width: 100%;
}

/* ── Form controls ───────────────────────────────────────────────────── */
.ctrl-input {
  min-height: 52px;
  padding: 10px 14px;
  background: var(--surface-3, #3f3f46);
  border: 1px solid var(--border-2, #52525b);
  border-radius: 10px;
  color: var(--text-1, #e4e4e7);
  font-size: 1rem;
  font-family: inherit;
  min-width: 0;
  box-sizing: border-box;
}

/* Compact centred number input – use for scores / timer digits */
.ctrl-input-num {
  width: 72px;
  flex: 0 0 72px !important; /* don't let ctrl-row override */
  text-align: center;
  padding-left: 4px;
  padding-right: 4px;
}

.ctrl-select {
  min-height: 52px;
  padding: 10px 14px;
  background: var(--surface-3, #3f3f46);
  border: 1px solid var(--border-2, #52525b);
  border-radius: 10px;
  color: var(--text-1, #e4e4e7);
  font-size: 1rem;
  font-family: inherit;
  min-width: 0;
  box-sizing: border-box;
}
`;

/** Idempotently inject the shared control stylesheet into <head>. */
export function injectControlStyles(): void {
	const MARKER_ID = 'plugin-control-styles';
	if (document.getElementById(MARKER_ID)) return;
	const style = document.createElement('style');
	style.id = MARKER_ID;
	style.textContent = CONTROL_STYLES;
	document.head.appendChild(style);
}
