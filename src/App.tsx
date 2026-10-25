import { useEffect, useRef } from "react";
import { useApp, type View } from "./store";
import { Logo, Meter, NavIcon, SearchIcon } from "./components/ui";
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
      <nav className="nav" aria-label={t.menu}>
        {NAV.map((n) => (
          <button type="button" key={n.v} aria-current={parent(view) === n.v ? "page" : undefined} onClick={() => go(n.v)}>
            <NavIcon name={n.key} />{t.nav[n.key]}
            {n.v === "issues" && report && <span className="badge">{flagged}</span>}
            {n.v === "cancel" && cancelN > 0 && <span className="badge">{cancelN}</span>}
          </button>
        ))}
      </nav>
      <div className="side-foot">
        {settings.auto_scan ? (
          <><span className="label">{t.autoOn(`${t.sched[settings.schedule]} ${settings.schedule_time}`)}</span>{nr && <span className="value">{t.nextRun(t.dateTime(nr))}</span>}</>
        ) : (
          <><span className="label">{t.autoOff}</span><span className="sub">{t.autoOffSub}</span></>
        )}
        <span className="sub">{report ? t.vaults(vaults) : boot.version ? `1Password · v${boot.version}` : "—"}</span>
      </div>
    </aside>
  );
}

function Topbar() {
  const { t, view, go, scan, scanning, state, query, setQuery, setListFilter } = useApp();
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k" && !document.querySelector(".modal-bg")) { e.preventDefault(); ref.current?.focus(); }
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
          onChange={async (e) => {
            const v = e.target.value;
            // ほかの画面からの検索は、一覧へ移れたときだけ文字を入れる（離れる確認で止めたら、検索欄も元のまま）
            if (view !== "issues" && !(await go("issues"))) return;
            setQuery(v); setListFilter(null);
          }} />
        <kbd>Ctrl K</kbd>
      </label>
      <button type="button" className={`chipbtn${pend ? " alert" : ""}`} onClick={() => go("settings")}>{pend ? t.aiRequests(pend) : t.mcpChip}</button>
      <button type="button" className="btn" onClick={() => void scan()} disabled={scanning} aria-busy={scanning}>
        <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"><path d="M13.5 8 A5.5 5.5 0 1 1 11.8 4" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" /><path d="M12.5 1.5 V4.5 H9.5" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" /></svg>
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
      <b>{t.stage[progress.stage]}</b>{cnt && pct != null && <span className="m cool"> {cnt.done} / {cnt.total}</span>}
      <Meter value={(pct ?? (progress.stage === "lists" ? 90 : 15)) / 100} tone="cool" thin />
    </div>
  );
}

/** 押したボタンが消えたり画面が変わったりしたら、キーボードの位置を今の画面の見出しへ移す（ページの先頭へ戻らないように） */
function useFocusKeeper(view: string) {
  const last = useRef<Element | null>(null);
  const first = useRef(true);
  const toHeading = () => {
    const h = document.querySelector<HTMLElement>(".view h1");
    if (!h) return;
    h.tabIndex = -1;
    h.focus({ preventScroll: true });
  };
  useEffect(() => {
    const onIn = (e: FocusEvent) => { last.current = e.target as Element; };
    document.addEventListener("focusin", onIn);
    const mo = new MutationObserver(() => {
      const a = document.activeElement;
      if ((!a || a === document.body) && last.current && !last.current.isConnected && !document.querySelector(".modal-bg")) {
        last.current = null;
        toHeading();
      }
    });
    mo.observe(document.body, { childList: true, subtree: true });
    return () => { document.removeEventListener("focusin", onIn); mo.disconnect(); };
  }, []);
  useEffect(() => {
    if (first.current) { first.current = false; return; }
    // 画面を変えたのが検索欄への入力なら、入力を続けられるようにそのままにする
    if (document.activeElement?.closest(".tsearch")) return;
    requestAnimationFrame(toHeading);
  }, [view]);
}

export function App() {
  const { view, api, t } = useApp();
  useFocusKeeper(view);
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
