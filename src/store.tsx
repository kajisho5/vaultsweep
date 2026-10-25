// 画面全体で共有する状態
import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { Api } from "./lib/api";
import { DICTS, type Dict, type Lang } from "./lib/i18n";
import { mainKind, type ApiError, type Boot, type Item, type Kind, type Progress, type Report, type Settings, type State } from "./lib/types";
import { confetti } from "./lib/confetti";

export type View = "overview" | "issues" | "change" | "batch" | "cancel" | "cflow" | "history" | "settings";

export interface ModalButton { label: string; kind?: "primary" | "coral" | "ghost"; value: string }
export interface ModalSpec { title: string; body: string; cmd?: string; buttons: ModalButton[] }

interface Ctx {
  api: Api;
  boot: Boot;
  t: Dict;
  lang: Lang;
  report: Report | null;
  state: State;
  settings: Settings;
  nextRun: string | null;
  view: View;
  go(v: View): void;
  setReport(r: Report | null): void;
  setState(s: State): void;
  saveSettings(s: Settings): Promise<string | null>;
  toast(msg: string): void;
  ask(m: ModalSpec): Promise<string>;
  errText(e: unknown): string;
  fail(e: unknown): void;
  scan(): Promise<void>;
  scanning: boolean;
  progress: Progress | null;
  listFilter: Kind | null;
  setListFilter(k: Kind | null): void;
  query: string;
  setQuery(q: string): void;
  selId: string | null;
  setSelId(id: string | null): void;
  changeId: string | null;
  startChange(id: string): void;
  batchIds: string[];
  startBatch(ids: string[]): void;
  flowId: string | null;
  startFlow(id: string): void;
  todoIds: TodoEntry[];
  item(id: string | null): Item | undefined;
  confirmArchive(id: string, title: string): Promise<boolean>;
  celebrate(): void;
  leaveGuard: React.MutableRefObject<(() => Promise<boolean>) | null>;
}

const C = createContext<Ctx | null>(null);
export const useApp = () => {
  const c = useContext(C);
  if (!c) throw new Error("no ctx");
  return c;
};

function resolveTheme(t: Settings["theme"]) {
  if (t === "system") return matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  return t;
}
export function applyTheme(t: Settings["theme"]) {
  const r = resolveTheme(t);
  document.documentElement.dataset.theme = r;
  try { localStorage.setItem("vs-theme", t); } catch { /* 保存できなくても動く */ }
}

export interface TodoEntry { id: string; kind: Kind }
const topIds = (r: Report | null): TodoEntry[] => (r ? r.items.filter((i) => i.issues.length).slice(0, 3).map((i) => ({ id: i.id, kind: mainKind(i)! })) : []);

export function AppProvider({ api, boot: initial, children }: { api: Api; boot: Boot; children: ReactNode }) {
  const [boot] = useState(initial);
  const [report, setReport] = useState<Report | null>(initial.report);
  const [state, setState] = useState<State>(initial.state);
  const [settings, setSettings] = useState<Settings>(initial.settings);
  const [nextRun, setNextRun] = useState<string | null>(initial.next_run);
  const [view, setView] = useState<View>("overview");
  const [toastMsg, setToastMsg] = useState<{ m: string; on: boolean }>({ m: "", on: false });
  const [modal, setModal] = useState<(ModalSpec & { resolve(v: string): void }) | null>(null);
  const [scanning, setScanning] = useState(false);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [listFilter, setListFilter] = useState<Kind | null>(null);
  const [query, setQuery] = useState("");
  const [selId, setSelId] = useState<string | null>(null);
  const [changeId, setChangeId] = useState<string | null>(null);
  const [batchIds, setBatchIds] = useState<string[]>([]);
  const [flowId, setFlowId] = useState<string | null>(null);
  const [todoIds, setTodoIds] = useState<TodoEntry[]>(() => topIds(initial.report));
  const toastTimer = useRef<number | undefined>(undefined);
  const leaveGuard = useRef<(() => Promise<boolean>) | null>(null);

  const lang: Lang = settings.lang ?? boot.system_lang;
  const t = DICTS[lang];

  useEffect(() => { document.documentElement.lang = lang; }, [lang]);
  useEffect(() => {
    applyTheme(settings.theme);
    if (settings.theme !== "system") return;
    const mq = matchMedia("(prefers-color-scheme: dark)");
    const on = () => applyTheme("system");
    mq.addEventListener("change", on);
    return () => mq.removeEventListener("change", on);
  }, [settings.theme]);

  const toast = useCallback((m: string) => {
    setToastMsg({ m, on: true });
    clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToastMsg((x) => ({ ...x, on: false })), 2800);
  }, []);

  const ask = useCallback((m: ModalSpec) => new Promise<string>((resolve) => setModal({ ...m, resolve })), []);
  const closeModal = (v: string) => { modal?.resolve(v); setModal(null); };

  const errText = useCallback((e: unknown) => {
    const x = e as Partial<ApiError> | string;
    if (typeof x === "string") return x;
    const code = x?.code ?? "other";
    const base = t.err[code] ?? t.err.other;
    const detail = "detail" in (x as object) ? (x as { detail?: string }).detail : undefined;
    return code === "other" && detail ? `${base}（${detail}）` : base;
  }, [t]);

  const fail = useCallback((e: unknown) => {
    const code = (e as ApiError)?.code;
    if (code === "not_installed" || code === "not_signed_in") {
      void ask({ title: t.setupTitle, body: errText(e) + "\n\n" + t.firstSteps.map((s, i) => `${i + 1}. ${s}`).join("\n"), buttons: [{ label: t.setupLink, kind: "ghost", value: "link" }, { label: t.ok, kind: "primary", value: "ok" }] })
        .then((v) => { if (v === "link") void api.openUrl("https://www.1password.dev/cli/get-started"); });
    } else toast(errText(e));
  }, [api, ask, errText, t, toast]);

  const go = useCallback(async (v: View) => {
    if (leaveGuard.current) {
      const ok = await leaveGuard.current();
      if (!ok) return;
      leaveGuard.current = null;
    }
    setView(v);
    window.scrollTo({ top: 0 });
  }, []);

  const scan = useCallback(async () => {
    if (scanning) return;
    setScanning(true);
    setProgress({ stage: "connecting" });
    try {
      const res = await api.runScan(setProgress);
      setReport(res.report);
      setTodoIds(topIds(res.report));
      const b = await api.bootstrap();
      setState(b.state);
      toast(t.scanDone(res.new_issues));
    } catch (e) {
      fail(e);
    } finally {
      setScanning(false);
      setProgress(null);
    }
  }, [api, fail, scanning, t, toast]);

  const saveSettings = useCallback(async (s: Settings) => {
    setSettings(s);
    try {
      const r = await api.saveSettings(s);
      setSettings(r.settings);
      setNextRun(r.next_run);
      return r.schedule_error;
    } catch (e) {
      fail(e);
      return null;
    }
  }, [api, fail]);

  const item = useCallback((id: string | null) => (id ? report?.items.find((i) => i.id === id) : undefined), [report]);

  const confirmArchive = useCallback(async (id: string, title: string) => {
    const v = await ask({ title: t.mTitle, body: t.mBody(title), cmd: "op item delete <ID> --archive", buttons: [{ label: t.cancel, kind: "ghost", value: "no" }, { label: t.archiveBtn, kind: "coral", value: "yes" }] });
    if (v !== "yes") return false;
    try {
      const st = await api.archive(id);
      setState(st);
      const b = await api.bootstrap();
      setReport(b.report);
      toast(t.archived);
      return true;
    } catch (e) {
      fail(e);
      return false;
    }
  }, [api, ask, fail, t, toast]);

  // AIアプリからの依頼や自動診断の結果を拾う（アプリに戻ったときと30秒ごと）
  useEffect(() => {
    if (api.demo) return;
    const pull = async () => {
      try {
        const b = await api.bootstrap();
        setState(b.state);
        setReport((r) => (b.report && (!r || b.report.scanned_at !== r.scanned_at) ? b.report : r));
      } catch { /* 次の機会に */ }
    };
    const id = window.setInterval(pull, 30000);
    window.addEventListener("focus", pull);
    return () => { clearInterval(id); window.removeEventListener("focus", pull); };
  }, [api]);

  const value: Ctx = useMemo(() => ({
    api, boot, t, lang, report, state, settings, nextRun, view, go, setReport, setState, saveSettings, toast, ask, errText, fail, scan, scanning, progress,
    listFilter, setListFilter, query, setQuery, selId, setSelId,
    changeId, startChange: (id: string) => { setChangeId(id); void go("change"); },
    batchIds, startBatch: (ids: string[]) => { setBatchIds(ids); void go("batch"); },
    flowId, startFlow: (id: string) => { setFlowId(id); void go("cflow"); },
    todoIds, item, confirmArchive, celebrate: confetti, leaveGuard,
  }), [api, boot, t, lang, report, state, settings, nextRun, view, go, saveSettings, toast, ask, errText, fail, scan, scanning, progress, listFilter, query, selId, changeId, batchIds, flowId, todoIds, item, confirmArchive]);

  return (
    <C.Provider value={value}>
      {children}
      {modal && (
        <div className="modal-bg" onClick={(e) => { if (e.target === e.currentTarget) closeModal(""); }} onKeyDown={(e) => { if (e.key === "Escape") closeModal(""); }}>
          <div className="modal" role="dialog" aria-modal="true" aria-labelledby="mTitle">
            <h2 id="mTitle">{modal.title}</h2>
            <p style={{ whiteSpace: "pre-line" }}>{modal.body}</p>
            {modal.cmd && <div className="cmd mono">{modal.cmd}</div>}
            <div className="btns">
              {modal.buttons.map((b, i) => (
                <button key={b.value} className={`btn ${b.kind ?? "ghost"}`} autoFocus={i === modal.buttons.length - 1} onClick={() => closeModal(b.value)}>{b.label}</button>
              ))}
            </div>
          </div>
        </div>
      )}
      <div className={`toast${toastMsg.on ? " on" : ""}`} role="status" aria-live="polite">{toastMsg.m}</div>
      <canvas id="confetti" aria-hidden="true" />
    </C.Provider>
  );
}
