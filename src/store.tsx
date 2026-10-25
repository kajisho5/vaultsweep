// 画面全体で共有する状態
import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { Api } from "./lib/api";
import { DICTS, MAC, type Dict, type Lang } from "./lib/i18n";
import { mainKind, type ApiError, type Boot, type Item, type Kind, type Progress, type Report, type Settings, type State } from "./lib/types";

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
  go(v: View): Promise<boolean>;
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
const noop = () => {};

export function applyTheme(t: Settings["theme"]) {
  const r = resolveTheme(t);
  document.documentElement.dataset.theme = r;
  try { localStorage.setItem("vs-theme", t); } catch { /* 保存できなくても動く */ }
}

export interface TodoEntry { id: string; kind: Kind }
const topIds = (r: Report | null): TodoEntry[] => (r ? r.items.filter((i) => i.issues.length).slice(0, 3).map((i) => ({ id: i.id, kind: mainKind(i)! })) : []);

export function AppProvider({ api, boot: initial, children }: { api: Api; boot: Boot; children: ReactNode }) {
  const [boot] = useState(initial);
  const [report, setReportRaw] = useState<Report | null>(initial.report);
  const [state, setStateRaw] = useState<State>(initial.state);
  // 操作で結果が変わるたびに数を進める。裏での読み直しは、その間に操作があったら捨てる（新しい結果を古い結果で上書きしない）
  const ver = useRef(0);
  const setReport = useCallback((v: Report | null | ((r: Report | null) => Report | null)) => { ver.current++; setReportRaw(v); }, []);
  const setState = useCallback((v: State | ((s: State) => State)) => { ver.current++; setStateRaw(v); }, []);
  const reportRef = useRef(report);
  reportRef.current = report;
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
  const t = useMemo<Dict>(() => (boot.os === "macos" ? { ...DICTS[lang], ...MAC[lang] } : DICTS[lang]), [boot.os, lang]);

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
    // 長い知らせほど長く出しておく
    toastTimer.current = window.setTimeout(() => setToastMsg((x) => ({ ...x, on: false })), Math.min(9000, 2800 + m.length * 45));
  }, []);

  const modalRef = useRef<(ModalSpec & { resolve(v: string): void }) | null>(null);
  const ask = useCallback((m: ModalSpec) => new Promise<string>((resolve) => {
    // 前の確認が開いたままなら「何も選ばなかった」で閉じる
    modalRef.current?.resolve("");
    const next = { ...m, resolve };
    modalRef.current = next;
    setModal(next);
  }), []);
  const closeModal = useCallback((v: string) => {
    const cur = modalRef.current;
    modalRef.current = null;
    setModal(null);
    cur?.resolve(v);
  }, []);

  const errText = useCallback((e: unknown) => {
    if (e == null) return t.err.other;
    // 文字のままのエラー（中の決まりごとの英文など）は、そのまま見せず一言に添える
    if (typeof e === "string") return t.err.other + t.paren(e.length > 120 ? e.slice(0, 120) + "…" : e);
    const x = e as Partial<ApiError> & { detail?: string };
    const code = x.code ?? "other";
    const detail = typeof x.detail === "string" ? x.detail : undefined;
    if (code === "not_signed_in" && detail === "timeout") return t.err.timeout;
    const base = t.err[code] ?? t.err.other;
    // 中身の決まった合図（no_session など）は見せない
    return code === "other" && detail && !/^[a-z_]+$/.test(detail) ? base + t.paren(detail) : base;
  }, [t]);

  const fail = useCallback((e: unknown) => {
    const code = (e as ApiError)?.code;
    const timeout = (e as { detail?: string })?.detail === "timeout";
    if (code === "not_installed" || (code === "not_signed_in" && !timeout)) {
      void ask({ title: t.setupTitle, body: errText(e) + "\n\n" + t.firstSteps.map((s, i) => `${i + 1}. ${s}`).join("\n"), buttons: [{ label: t.setupLink, kind: "ghost", value: "link" }, { label: t.ok, kind: "primary", value: "ok" }] })
        .then((v) => { if (v === "link") void api.openUrl("https://www.1password.dev/cli/get-started"); });
    } else toast(errText(e));
  }, [api, ask, errText, t, toast]);

  const go = useCallback(async (v: View) => {
    if (leaveGuard.current) {
      const ok = await leaveGuard.current();
      if (!ok) return false;
      leaveGuard.current = null;
    }
    setView(v);
    window.scrollTo({ top: 0 });
    return true;
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

  // 保存は順番に1つずつ。失敗したら画面の値を元に戻す
  const saveQueue = useRef<Promise<unknown>>(Promise.resolve());
  const settingsRef = useRef(settings);
  settingsRef.current = settings;
  const saveSettings = useCallback((s: Settings) => {
    const prev = settingsRef.current;
    setSettings(s);
    const job = saveQueue.current.then(async () => {
      try {
        const r = await api.saveSettings(s);
        setSettings(r.settings);
        setNextRun(r.next_run);
        return r.schedule_error;
      } catch (e) {
        setSettings(prev);
        fail(e);
        return null;
      }
    });
    saveQueue.current = job.catch(() => {});
    return job;
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
        const v0 = ver.current;
        const b = await api.bootstrap();
        if (ver.current !== v0) return;
        setStateRaw(b.state);
        setReportRaw((r) => {
          if (b.report && (!r || b.report.scanned_at !== r.scanned_at)) setTodoIds(topIds(b.report));
          return b.report ?? r;
        });
      } catch { /* 次の機会に */ }
    };
    const id = window.setInterval(pull, 30000);
    window.addEventListener("focus", pull);
    return () => { clearInterval(id); window.removeEventListener("focus", pull); };
  }, [api]);

  // 作ったまま確定していない項目があるのにウィンドウを閉じようとしたとき
  useEffect(() => {
    let un: (() => void) | null = null;
    let dead = false;
    void api.onCloseGuard(async (ids) => {
      const names = ids.map((id) => reportRef.current?.items.find((i) => i.id === id)?.title ?? id).join(t.dot);
      const v = await ask({ title: t.closeTitle, body: t.closeBody(names), buttons: [
        { label: t.closeQuit, kind: "ghost", value: "quit" },
        { label: t.closeOpen, kind: "ghost", value: "open" },
        { label: t.closeStay, kind: "primary", value: "stay" },
      ] });
      if (v === "quit") void api.quitNow();
      else if (v === "open" && ids[0]) { setChangeId(ids[0]); void go("change"); }
    }).then((f) => { if (dead) f(); else un = f; });
    return () => { dead = true; un?.(); };
  }, [api, ask, go, t]);

  const value: Ctx = useMemo(() => ({
    api, boot, t, lang, report, state, settings, nextRun, view, go, setReport, setState, saveSettings, toast, ask, errText, fail, scan, scanning, progress,
    listFilter, setListFilter, query, setQuery, selId, setSelId,
    changeId, startChange: (id: string) => { setChangeId(id); void go("change"); },
    batchIds, startBatch: (ids: string[]) => { setBatchIds(ids); void go("batch"); },
    flowId, startFlow: (id: string) => { setFlowId(id); void go("cflow"); },
    todoIds, item, confirmArchive, celebrate: noop, leaveGuard,
  }), [api, boot, t, lang, report, state, settings, nextRun, view, go, saveSettings, toast, ask, errText, fail, scan, scanning, progress, listFilter, query, selId, changeId, batchIds, flowId, todoIds, item, confirmArchive]);

  return (
    <C.Provider value={value}>
      {children}
      {modal && <Modal spec={modal} close={closeModal} />}
      <div className={`toast${toastMsg.on ? " on" : ""}`} role="status" aria-live="polite">{toastMsg.m}</div>
    </C.Provider>
  );
}

/** 確認の画面。Esc で閉じる・Tab は中だけを回る・閉じたら元の場所に戻る・最初は「やめる」側に合わせる */
function Modal({ spec, close }: { spec: ModalSpec; close(v: string): void }) {
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const before = document.activeElement as HTMLElement | null;
    box.current?.querySelector<HTMLButtonElement>("button")?.focus();
    const on = (e: KeyboardEvent) => {
      if (e.key === "Escape") { e.preventDefault(); close(""); return; }
      if (e.key !== "Tab" || !box.current) return;
      const bs = [...box.current.querySelectorAll<HTMLElement>("button")];
      if (!bs.length) return;
      const i = bs.indexOf(document.activeElement as HTMLElement);
      if (e.shiftKey && i <= 0) { e.preventDefault(); bs[bs.length - 1].focus(); }
      else if (!e.shiftKey && (i === -1 || i === bs.length - 1)) { e.preventDefault(); bs[0].focus(); }
    };
    document.addEventListener("keydown", on, true);
    return () => { document.removeEventListener("keydown", on, true); before?.focus?.(); };
  }, [spec, close]);
  return (
    <div className="modal-bg" onClick={(e) => { if (e.target === e.currentTarget) close(""); }}>
      <div className="modal" role="dialog" aria-modal="true" aria-labelledby="mTitle" ref={box}>
        <h2 id="mTitle">{spec.title}</h2>
        <p style={{ whiteSpace: "pre-line" }}>{spec.body}</p>
        {spec.cmd && <div className="cmd m">{spec.cmd}</div>}
        <div className="btns">
          {spec.buttons.map((b) => (
            <button key={b.value} type="button" className={`btn${b.kind && b.kind !== "ghost" ? ` ${b.kind}` : ""}`} onClick={() => close(b.value)}>{b.label}</button>
          ))}
        </div>
      </div>
    </div>
  );
}
