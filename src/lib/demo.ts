// デモ用の中身。名前・サイト・数字はすべて架空。1Password にはつながない。
import type { Api } from "./api";
import type { Boot, CancelEntry, Issue, Item, Kind, Pending, Report, Settings, State } from "./types";
import { KINDS, PASSWORD_KINDS, SEVERITY } from "./types";

function rng(seed: number) {
  return () => {
    seed |= 0; seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export function riskOf(issues: Issue[]): number {
  if (!issues.length) return 0;
  const s = issues.map((i) => SEVERITY[i.kind]).sort((a, b) => b - a);
  let r = s[0] + 0.06 * (s.length - 1);
  const leak = issues.find((i) => i.kind === "leak");
  if (leak && leak.kind === "leak" && leak.count >= 1000) r += 0.04;
  return Math.round(Math.min(0.99, r) * 100) / 100;
}
export function scoreOf(items: Item[]): number {
  if (!items.length) return 100;
  const pen = items.reduce((a, i) => a + i.risk, 0);
  return Math.round(Math.max(0, Math.min(100, 100 * (1 - (pen / items.length) * 1.6))));
}
function recompute(r: Report) {
  for (const it of r.items) {
    it.issues.sort((a, b) => SEVERITY[b.kind] - SEVERITY[a.kind]);
    it.risk = riskOf(it.issues);
  }
  r.items.sort((a, b) => b.risk - a.risk || a.title.localeCompare(b.title));
  r.total = r.items.length;
  r.counts = Object.fromEntries(KINDS.map((k) => [k, r.items.filter((i) => i.issues.some((x) => x.kind === k)).length]));
  r.score = scoreOf(r.items);
}

const pad = (n: number, w = 2) => String(n).padStart(w, "0");
const iso = (d: Date) => {
  const off = -d.getTimezoneOffset();
  const s = off >= 0 ? "+" : "-";
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}${s}${pad(Math.floor(Math.abs(off) / 60))}:${pad(Math.abs(off) % 60)}`;
};

type Known = [string, string, Kind[], { leak?: number; reuse?: number }?];
const KNOWN: Record<number, Known> = {
  17: ["サンプル通販A", "shop-a.example", ["leak", "reuse", "nofa"], { leak: 1532, reuse: 1 }],
  42: ["練習用フォーラムE", "forum-e.example", ["leak", "weak", "stale"], { leak: 88 }],
  8: ["テスト動画サブスク", "video-c.example", ["breach", "stale"]],
  63: ["ダミー銀行（架空）", "bank-d.example", ["similar", "weak", "nofa"]],
  29: ["架空SNS-B", "sns-b.example", ["reuse", "nofa"], { reuse: 1 }],
  91: ["サンプル旅行予約F", "travel-f.example", ["reuse"], { reuse: 1 }],
  105: ["架空クラウドG", "cloud-g.example", ["stale", "dup"]],
  120: ["テスト求人H", "jobs-h.example", ["dup"]],
  77: ["サンプル学習塾K", "juku-k.example", ["similar", "stale"]],
  130: ["架空ポイントL", "point-l.example", ["similar"]],
};

function build(now: Date): { report: Report; state: State } {
  const R = rng(42);
  const N = 143;
  const extra: number[] = [];
  for (let i = 1; i <= N; i++) if (!KNOWN[i]) extra.push(i);
  for (let i = extra.length - 1; i > 0; i--) { const j = Math.floor(R() * (i + 1)); [extra[i], extra[j]] = [extra[j], extra[i]]; }
  const flagged = new Set(extra.slice(0, 38 - Object.keys(KNOWN).length));
  const pick: Kind[] = ["reuse", "reuse", "weak", "weak", "stale", "stale", "stale", "nofa", "dup", "leak", "breach", "reuse", "stale", "weak", "nofa"];
  const raw: { n: number; title: string; dom: string; kinds: Kind[]; leak?: number; group?: number; days: number }[] = [];
  for (let i = 1; i <= N; i++) {
    let title: string, dom: string, kinds: Kind[] = [], leak: number | undefined, group: number | undefined;
    if (KNOWN[i]) { const k = KNOWN[i]; [title, dom, kinds] = k; leak = k[3]?.leak; group = k[3]?.reuse; }
    else {
      title = `架空サイト${pad(i, 3)}`; dom = `site-${i}.example`;
      if (flagged.has(i)) {
        kinds.push(pick[Math.floor(R() * pick.length)]);
        if (R() < 0.3) { const k = pick[Math.floor(R() * pick.length)]; if (!kinds.includes(k)) kinds.push(k); }
      }
    }
    if (kinds.includes("leak") && leak == null) leak = 3 + Math.floor(R() * 400);
    const days = kinds.includes("stale") ? 400 + Math.floor(R() * 1100) : 10 + Math.floor(R() * 340);
    raw.push({ n: i, title, dom, kinds, leak, group, days: i === 17 ? 936 : days });
  }
  // 使い回しのまとまり（17・29・91 は 1 番。ほかは2件ずつ）
  let g = 1;
  const loose = raw.filter((x) => x.kinds.includes("reuse") && x.group == null);
  for (let i = 0; i < loose.length; i += 2) { g++; loose[i].group = g; if (loose[i + 1]) loose[i + 1].group = g; else loose[i].group = 1; }
  const byGroup = new Map<number, typeof raw>();
  for (const x of raw) if (x.group) byGroup.set(x.group, [...(byGroup.get(x.group) ?? []), x]);
  const id = (n: number) => `demo${pad(n, 3)}${"x".repeat(19)}`;
  const items: Item[] = raw.map((x) => {
    const upd = new Date(now.getTime() - x.days * 86400000);
    const issues: Issue[] = x.kinds.map((k): Issue => {
      switch (k) {
        case "leak": return { kind: "leak", count: x.leak ?? 10 };
        case "breach": return { kind: "breach", breach: "Sample Video Service", date: "2024-03-12" };
        case "reuse": { const others = (byGroup.get(x.group!) ?? []).filter((o) => o.n !== x.n); return { kind: "reuse", group: x.group!, others: others.slice(0, 3).map((o) => o.title), total: others.length }; }
        case "similar": { const others = raw.filter((o) => o.n !== x.n && o.kinds.includes("similar")); return { kind: "similar", group: 1, others: others.slice(0, 3).map((o) => o.title), total: others.length }; }
        case "weak": return { kind: "weak", strength: x.n % 2 ? "weak" : "terrible" };
        case "nofa": return { kind: "nofa", doc: `https://help.${x.dom}/2fa` };
        case "stale": return { kind: "stale", days: x.days };
        case "dup": { const other = x.n === 105 ? 120 : x.n === 120 ? 105 : x.n + 1; return { kind: "dup", other: raw[other - 1]?.title ?? "架空サイト", other_id: id(other) }; }
      }
    });
    const del = x.n === 105
      ? { url: null, difficulty: "hard", email: "support@cloud-g.example", email_subject: "Account deletion request", email_body: "I want my account to be deleted. My user name is XXXXXX.", notes: null }
      : x.n % 3 === 0 || x.n === 8 || x.n === 42 ? { url: `https://${x.dom}/account/delete`, difficulty: x.n % 2 ? "easy" : "medium", email: null, email_subject: null, email_body: null, notes: "Settings → Account → Delete account" } : null;
    return {
      id: id(x.n), title: x.title, vault: x.n % 7 === 0 ? "Shared" : x.n % 5 === 0 ? "Work" : "Private",
      domain: x.dom, url: `https://${x.dom}/login`, updated_at: iso(upd), days_since_update: x.days, issues, risk: 0,
      has_totp: !x.kinds.includes("nofa") && x.n % 4 === 0, passkey_site: x.n === 63 || x.n % 11 === 0,
      twofa_doc: x.kinds.includes("nofa") ? `https://help.${x.dom}/2fa` : null, deletion: del,
      username_hint: x.title.length % 2 ? "ta•••" : "us•••",
    };
  });
  const report: Report = {
    scanned_at: iso(new Date(now.getTime() - 5.5 * 3600000)), total: N, score: 0, counts: {}, items,
    sources: { pwned_passwords: true, breaches: true, twofa: true, passkeys: true, justdeleteme: true }, warnings: [], email_breaches: [],
  };
  recompute(report);
  // 履歴：12週ぶん
  const history = [];
  const deltas = [-2, 1, -4, -2, 1, -2, -2, 1, -3, 2, -4];
  let s = report.score; const scores = [s];
  for (const d of deltas) { s += d; scores.unshift(s); }
  const flaggedNow = report.items.filter((i) => i.issues.length).length;
  const news = [0, 1, 0, 0, 2, 0, 1, 0, 0, 1, 1, 2];
  const monday = new Date(now); monday.setHours(9, 0, 0, 0); monday.setDate(monday.getDate() - ((monday.getDay() + 6) % 7));
  for (let i = 0; i < 12; i++) {
    const d = new Date(monday.getTime() - (11 - i) * 7 * 86400000);
    history.push({ at: iso(d), score: scores[i], flagged: flaggedNow + (11 - i) + (i % 3 === 0 ? 1 : 0), new_issues: news[i] });
  }
  history[11].at = report.scanned_at;
  const ago = (d: number) => iso(new Date(now.getTime() - d * 86400000));
  const cancel: Record<string, CancelEntry> = {
    [id(8)]: { state: "todo", title: "テスト動画サブスク", domain: "video-c.example", added_at: ago(3), left_at: null, archived_at: null, receipt: null },
    [id(105)]: { state: "todo", title: "架空クラウドG", domain: "cloud-g.example", added_at: ago(2), left_at: null, archived_at: null, receipt: null },
    [id(42)]: { state: "left", title: "練習用フォーラムE", domain: "forum-e.example", added_at: ago(6), left_at: ago(0), archived_at: null, receipt: null },
    demo900xxxxxxxxxxxxxxxxxxx: { state: "archived", title: "サンプル写真共有J", domain: "photo-j.example", added_at: ago(9), left_at: ago(2), archived_at: ago(1), receipt: null },
  };
  const fixed = [1, 2, 3, 5].map((d, i) => ({ at: ago(d), id: `demo8${i}0xxxxxxxxxxxxxxxxxxxx`, title: `架空サイト${pad(130 + i, 3)}` }));
  const pending: Pending[] = [{ id: "demo-req-1", kind: "archive", item_id: id(120), title: "テスト求人H", at: ago(0), from: "mcp" }];
  return { report, state: { in_use: [], cancel, history, fixed, pending } };
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export function demoApi(): Api {
  const now = new Date();
  const db = build(now);
  let settings: Settings = {
    lang: null, theme: "system", auto_scan: true, schedule: "weekly", schedule_time: "09:00", notify: "new", offline: false,
    stale_days: 365, weak_level: "weak", pw_length: 24, pw_symbols: true, hibp_key_ref: null,
  };
  const sessions = new Map<string, { generated: boolean }>();
  const clone = <T,>(x: T): T => structuredClone(x);
  const nextRun = () => {
    if (!settings.auto_scan) return null;
    const [h, m] = settings.schedule_time.split(":").map(Number);
    const d = new Date(); d.setSeconds(0, 0);
    for (let i = 0; i < 62; i++) {
      const c = new Date(d); c.setDate(d.getDate() + i); c.setHours(h, m, 0, 0);
      const ok = settings.schedule === "daily" || (settings.schedule === "monthly" ? c.getDate() === 1 : c.getDay() === 1);
      if (ok && c > new Date()) return `${c.getFullYear()}-${pad(c.getMonth() + 1)}-${pad(c.getDate())}T${pad(h)}:${pad(m)}`;
    }
    return null;
  };
  const find = (id: string) => db.report.items.find((i) => i.id === id);
  const sysLang = (navigator.language || "en").toLowerCase().startsWith("ja") ? "ja" : "en";
  const removeItem = (id: string) => { db.report.items = db.report.items.filter((i) => i.id !== id); recompute(db.report); };
  return {
    demo: true,
    async bootstrap(): Promise<Boot> {
      return { settings: clone(settings), state: clone(db.state), report: clone(db.report), data_dir: "(demo)", mcp_path: "C:\\Users\\sample\\AppData\\Local\\vaultsweep\\vaultsweep-mcp.exe", next_run: nextRun(), os: "demo", version: "0.1.0", system_lang: sysLang };
    },
    async checkConnection() { await sleep(300); return { version: "2.41.1", vaults: 3, account: "sample.1password.example" }; },
    async runScan(on) {
      on({ stage: "connecting" }); await sleep(300);
      for (let i = 0; i <= 143; i += 13) { on({ stage: "reading", done: Math.min(i, 143), total: 143 }); await sleep(40); }
      for (let i = 0; i <= 120; i += 12) { on({ stage: "checking", done: Math.min(i, 120), total: 120 }); await sleep(40); }
      on({ stage: "lists" }); await sleep(250);
      db.report.scanned_at = iso(new Date());
      db.state.history.push({ at: db.report.scanned_at, score: db.report.score, flagged: db.report.items.filter((i) => i.issues.length).length, new_issues: 0 });
      on({ stage: "done" });
      return { report: clone(db.report), new_issues: 0 };
    },
    async saveSettings(s) { settings = clone(s); return { settings: clone(settings), next_run: nextRun(), schedule_error: null }; },
    async setInUse(id, on) {
      if (on) { if (!db.state.in_use.includes(id)) db.state.in_use.push(id); const it = find(id); if (it) it.issues = it.issues.filter((x) => x.kind !== "stale"); recompute(db.report); }
      else db.state.in_use = db.state.in_use.filter((x) => x !== id);
      return clone(db.report);
    },
    async cancelAdd(id) {
      const it = find(id);
      db.state.cancel[id] ??= { state: "todo", title: it?.title ?? id, domain: it?.domain ?? null, added_at: iso(new Date()), left_at: null, archived_at: null, receipt: null };
      return clone(db.state);
    },
    async cancelSet(id, state, receipt = null) {
      if (state === "remove") delete db.state.cancel[id];
      else if (db.state.cancel[id]) {
        const e = db.state.cancel[id]; e.state = state;
        if (state === "left") e.left_at = iso(new Date());
        if (state === "archived") e.archived_at = iso(new Date());
        if (receipt) e.receipt = receipt;
      }
      return clone(db.state);
    },
    async changeBegin(id) { await sleep(250); sessions.set(id, { generated: false }); },
    async changeGenerate(id) { await sleep(450); const s = sessions.get(id); if (!s) throw { code: "other", detail: "no_session" }; s.generated = true; },
    async copySecret(id, which) {
      const s = sessions.get(id); if (!s || (which === "new" && !s.generated)) throw { code: "other", detail: "no_value" };
      return { seconds: 30 };
    },
    async copyPlain(text) { try { await navigator.clipboard.writeText(text); } catch { /* デモでは無視 */ } },
    async changeConfirm(id) {
      sessions.delete(id);
      const it = find(id); if (!it) return clone(db.report);
      const grouped = it.issues.filter((x) => x.kind === "reuse" || x.kind === "similar");
      it.issues = it.issues.filter((x) => !PASSWORD_KINDS.includes(x.kind));
      for (const gi of grouped) {
        if (gi.kind !== "reuse" && gi.kind !== "similar") continue;
        const mates = db.report.items.filter((i) => i.issues.some((x) => x.kind === gi.kind && x.group === gi.group));
        for (const m of mates) {
          const others = mates.filter((o) => o !== m).map((o) => o.title);
          if (!others.length) m.issues = m.issues.filter((x) => x.kind !== gi.kind);
          else for (const x of m.issues) if ((x.kind === "reuse" || x.kind === "similar") && x.kind === gi.kind) { x.others = others.slice(0, 3); x.total = others.length; }
        }
      }
      recompute(db.report);
      db.state.fixed.push({ at: iso(new Date()), id, title: it.title });
      return clone(db.report);
    },
    async changeRestore(id, noPasskey) { await sleep(300); const it = find(id); if (it?.passkey_site || !noPasskey) throw { code: "passkey_risk" }; const s = sessions.get(id); if (s) s.generated = false; },
    async changeEnd(id) { sessions.delete(id); },
    async archive(id) {
      await sleep(400);
      removeItem(id);
      if (db.state.cancel[id]) { db.state.cancel[id].state = "archived"; db.state.cancel[id].archived_at = iso(new Date()); }
      return clone(db.state);
    },
    async writeReceipt(id, _dl, _d, _rl, receipt) {
      await sleep(250);
      const e = db.state.cancel[id]; if (e) { e.state = "left"; e.left_at ??= iso(new Date()); if (receipt) e.receipt = receipt; }
      return clone(db.state);
    },
    async openUrl(url) { console.info("[demo] open", url); },
    async pendingResolve(id, approve) {
      const p = db.state.pending.find((x) => x.id === id) ?? null;
      db.state.pending = db.state.pending.filter((x) => x.id !== id);
      if (p && approve && p.kind === "archive") removeItem(p.item_id);
      return { pending: p, state: clone(db.state) };
    },
    async exportCsv() { return "-"; },
    async testKeyRef(r) { await sleep(300); return r.startsWith("op://"); },
  };
}
