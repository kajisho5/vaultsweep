// 中身（Rust）とやりとりするデータの形。パスワードは含まない。

export type Kind = "leak" | "breach" | "reuse" | "weak" | "nofa" | "stale" | "dup";
export const KINDS: Kind[] = ["leak", "breach", "reuse", "weak", "nofa", "stale", "dup"];
export const SEVERITY: Record<Kind, number> = { leak: 0.9, breach: 0.82, reuse: 0.7, weak: 0.62, nofa: 0.5, stale: 0.42, dup: 0.36 };
export const PASSWORD_KINDS: Kind[] = ["leak", "breach", "reuse", "weak"];

export type Issue =
  | { kind: "leak"; count: number }
  | { kind: "breach"; breach: string; date: string }
  | { kind: "reuse"; group: number; others: string[]; total: number }
  | { kind: "weak"; strength: string }
  | { kind: "nofa"; doc: string | null }
  | { kind: "stale"; days: number }
  | { kind: "dup"; other: string; other_id: string };

export interface Deletion {
  url: string | null;
  difficulty: string;
  email: string | null;
  email_subject: string | null;
  email_body: string | null;
  notes: string | null;
}

export interface Item {
  id: string;
  title: string;
  vault: string;
  domain: string | null;
  url: string | null;
  updated_at: string | null;
  days_since_update: number | null;
  issues: Issue[];
  risk: number;
  has_totp: boolean;
  passkey_site: boolean;
  twofa_doc: string | null;
  deletion: Deletion | null;
  username_hint: string | null;
}

export interface Report {
  scanned_at: string;
  total: number;
  score: number;
  counts: Partial<Record<Kind, number>>;
  items: Item[];
  sources: { pwned_passwords: boolean; breaches: boolean; twofa: boolean; passkeys: boolean; justdeleteme: boolean };
  warnings: string[];
  email_breaches: { masked: string; breaches: string[] }[];
}

export interface Settings {
  lang: "ja" | "en" | null;
  theme: "system" | "light" | "dark";
  auto_scan: boolean;
  schedule: "weekly" | "daily" | "monthly";
  schedule_time: string;
  notify: "new" | "always" | "never";
  offline: boolean;
  stale_days: number;
  weak_level: "weak" | "fair";
  pw_length: number;
  pw_symbols: boolean;
  hibp_key_ref: string | null;
}

export interface CancelEntry {
  state: "todo" | "left" | "archived";
  title: string;
  domain: string | null;
  added_at: string;
  left_at: string | null;
  archived_at: string | null;
  receipt: string | null;
}

export interface HistoryEntry { at: string; score: number; flagged: number; new_issues: number }
export interface FixedEntry { at: string; id: string; title: string }
export interface Pending { id: string; kind: "change" | "archive"; item_id: string; title: string; at: string; from: string }

export interface State {
  in_use: string[];
  cancel: Record<string, CancelEntry>;
  history: HistoryEntry[];
  fixed: FixedEntry[];
  pending: Pending[];
}

export interface Boot {
  settings: Settings;
  state: State;
  report: Report | null;
  data_dir: string;
  mcp_path: string | null;
  next_run: string | null;
  os: string;
  version: string;
  system_lang: "ja" | "en";
}

export type Progress =
  | { stage: "connecting" }
  | { stage: "reading"; done: number; total: number }
  | { stage: "checking"; done: number; total: number }
  | { stage: "lists" }
  | { stage: "emails"; done: number; total: number }
  | { stage: "done" };

export interface ScanResult { report: Report; new_issues: number }

export interface PayMatch { id: string; title: string; rows: number; total: number; last_date: string | null; stale: boolean }

/** 中身から返るエラー */
export type ApiError =
  | { code: "not_installed" }
  | { code: "not_signed_in"; detail: string }
  | { code: "bad_id" }
  | { code: "passkey_risk" }
  | { code: "parse"; detail: string }
  | { code: "other"; detail: string };

export function mainKind(it: Item): Kind | null {
  let best: Kind | null = null;
  for (const i of it.issues) if (!best || SEVERITY[i.kind] > SEVERITY[best]) best = i.kind;
  return best;
}
export const has = (it: Item, k: Kind) => it.issues.some((i) => i.kind === k);
