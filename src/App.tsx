import { useEffect, useRef } from "react";
import { useApp, type View } from "./store";
import { Logo, NavIcon, SearchIcon } from "./components/ui";
import { Overview } from "./views/Overview";
import { Issues } from "./views/Issues";
import { Change } from "./views/Change";
import { Batch } from "./views/Batch";
import { CancelBoard } from "./views/CancelBoard";
import { CancelFlow } from "./views/CancelFlow";
import { History } from "./views/History";
import { SettingsView } from "./views/Settings";

const NAV: { v: View; key: "overview" | "issues" | "cancel" | "history" | "settings" }[] = [
  { v: "overview", key: "overview" }, { v: "issues", key: "issues" }, { v: "cancel", key: "cancel" }, { v: "history", key: "history" }, { v: "settings", key: "settings" },
];

function parent(v: View): View {
  return v === "change" || v === "batch" ? "issues" : v === "cflow" ? "cancel" : v;
}

function Sidebar() {
  const { t, view, go, report, state, settings, nextRun, boot } = useApp();
  const flagged = report ? report.items.filter((i) => i.issues.length).length : 0;
  const cancelN = Object.values(state.cancel).filter((c) => c.state !== "archived").length;
  const vaults = report ? new Set(report.items.map((i) => i.vault)).size : 0;
  const nr = nextRun ? new Date(nextRun) : null;
  return (
    <aside className="side">
      <div className="logo"><Logo />vaultsweep</div>
      <div className="acct"><i>1P</i><div><b>1Password</b><small>{report ? t.vaults(vaults) : boot.version && `v${boot.version}`}</small></div></div>
      <nav className="nav" aria-label="menu">
        {NAV.map((n) => (
          <button key={n.v} aria-current={parent(view) === n.v ? "page" : undefined} onClick={() => go(n.v)}>
            <NavIcon name={n.key} />{t.nav[n.key]}
            {n.v === "issues" && report && <span className="badge">{flagged}</span>}
            {n.v === "cancel" && cancelN > 0 && <span className="badge">{cancelN}</span>}
          </button>
        ))}
      </nav>
      <div className="side-foot">
        {settings.auto_scan ? (
          <div className="auto"><b>{t.autoOn(`${t.sched[settings.schedule]} ${settings.schedule_time}`)}</b><span>{nr ? t.nextRun(t.dateTime(nr)) : ""}</span></div>
        ) : (
          <div className="auto off"><b>{t.autoOff}</b><span>{t.autoOffSub}</span></div>
        )}
      </div>
    </aside>
  );
}

function Topbar() {
  const { t, view, go, scan, scanning, state, query, setQuery, setListFilter } = useApp();
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") { e.preventDefault(); ref.current?.focus(); }
    };
    addEventListener("keydown", on);
    return () => removeEventListener("keydown", on);
  }, []);
  const crumb = view === "change" ? t.crumbChange : view === "batch" ? t.crumbBatch : view === "cflow" ? t.crumbFlow : t.nav[view as "overview"];
  const pend = state.pending.length;
  return (
    <header className="top">
      <div className="crumb">{crumb}</div>
      <div className="flex-1" />
      <label className="tsearch">
        <SearchIcon />
        <input ref={ref} value={query} placeholder={t.search} aria-label={t.search}
          onChange={(e) => { setQuery(e.target.value); setListFilter(null); if (view !== "issues") void go("issues"); }} />
        <kbd>Ctrl K</kbd>
      </label>
      <button className={`chipbtn${pend ? " alert" : ""}`} onClick={() => go("settings")}>{pend ? t.aiRequests(pend) : t.mcpChip}</button>
      <button className={`scan${scanning ? " busy" : ""}`} onClick={() => void scan()} disabled={scanning}>
        <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"><path d="M13.5 8 A5.5 5.5 0 1 1 11.8 4" fill="none" stroke="#fff" strokeWidth="1.8" strokeLinecap="round" /><path d="M12.5 1.5 V4.5 H9.5" fill="none" stroke="#fff" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" /></svg>
        <span>{scanning ? t.scanning : t.scan}</span>
      </button>
    </header>
  );
}

function ScanProgress() {
  const { progress, t } = useApp();
  if (!progress) return null;
  const cnt = "total" in progress ? progress : null;
  const pct = cnt && cnt.total ? Math.round((cnt.done / cnt.total) * 100) : null;
  return (
    <div className="overlay-scan" role="status" aria-live="polite">
      <b>{t.stage[progress.stage]}</b>{cnt && pct != null && <span style={{ color: "var(--ink3)" }}> {cnt.done} / {cnt.total}</span>}
      <div className="bar" style={{ margin: "8px 0 0" }}><b style={{ width: `${pct ?? (progress.stage === "lists" ? 90 : 15)}%` }} /></div>
    </div>
  );
}

export function App() {
  const { view, api, t } = useApp();
  return (
    <div className="app">
      <Sidebar />
      <div className="main">
        <Topbar />
        <ScanProgress />
        <div className="wrap">
          <section className="view" key={view}>
            {view === "overview" && <Overview />}
            {view === "issues" && <Issues />}
            {view === "change" && <Change />}
            {view === "batch" && <Batch />}
            {view === "cancel" && <CancelBoard />}
            {view === "cflow" && <CancelFlow />}
            {view === "history" && <History />}
            {view === "settings" && <SettingsView />}
          </section>
        </div>
        {api.demo && <p className="note">{t.demoNote}</p>}
      </div>
    </div>
  );
}
