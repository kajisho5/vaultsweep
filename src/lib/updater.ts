// 新しい版の確認と入れ替え。画面のどこからでも同じ状態を見られるように、小さな入れ物にまとめる。
import { useEffect, useSyncExternalStore } from "react";
import type { Api } from "./api";
import type { Settings } from "./types";

export type UpStatus = "idle" | "checking" | "latest" | "found" | "failed" | "offline" | "downloading" | "restarting" | "blocked" | "installFailed" | "demo";
export interface UpState {
  status: UpStatus;
  info: { version: string; current: string } | null;
  pct: number | null;
  /** 最後に確かめた日時（ISO） */
  last: string | null;
  /** 「あとで」を押した版（今回の起動中だけ隠す） */
  later: string | null;
}

const LAST_KEY = "vaultsweep.update.last";
const readLast = () => { try { return localStorage.getItem(LAST_KEY); } catch { return null; } };
const writeLast = (v: string) => { try { localStorage.setItem(LAST_KEY, v); } catch { /* 保存できなくても動く */ } };

let st: UpState = { status: "idle", info: null, pct: null, last: readLast(), later: null };
const subs = new Set<() => void>();
const set = (p: Partial<UpState>) => { st = { ...st, ...p }; subs.forEach((f) => f()); };
const subscribe = (f: () => void) => { subs.add(f); return () => { subs.delete(f); }; };

export function useUpdate(): UpState {
  return useSyncExternalStore(subscribe, () => st);
}

/** 確かめる。手で押したとき（manual）は、オフラインの設定でもその旨を見せる */
export async function checkUpdate(api: Api, offline: boolean) {
  if (st.status === "checking" || st.status === "downloading" || st.status === "restarting") return;
  if (offline) { set({ status: "offline" }); return; }
  set({ status: "checking" });
  try {
    const info = await api.updateCheck();
    const now = new Date().toISOString();
    writeLast(now);
    set({ status: info ? "found" : "latest", info, last: now });
  } catch (e) {
    const detail = (e as { detail?: string })?.detail;
    set({ status: detail === "offline" ? "offline" : "failed" });
  }
}

/** 入れて開き直す。パスワード変更の途中なら断る */
export async function installUpdate(api: Api) {
  if (!st.info || st.status === "downloading" || st.status === "restarting") return;
  try {
    if ((await api.changeUnfinished()).length) { set({ status: "blocked" }); return; }
  } catch { /* 確かめられなければ中身（Rust）側で断る */ }
  set({ status: "downloading", pct: 0 });
  try {
    await api.updateInstall((pct) => set({ pct, status: pct === 100 ? "restarting" : "downloading" }));
    set({ status: "restarting" });
  } catch (e) {
    const detail = (e as { detail?: string })?.detail;
    set({ status: detail === "change_unfinished" ? "blocked" : detail === "demo_update" ? "demo" : "installFailed", pct: null });
  }
}

export function laterUpdate() { if (st.info) set({ later: st.info.version }); }

/** 自動で確かめる：起動して1分後と、そのあと6時間ごと */
export function useAutoUpdate(api: Api, settings: Settings) {
  const on = settings.auto_update && !settings.offline;
  useEffect(() => {
    if (!on) return;
    const first = setTimeout(() => void checkUpdate(api, false), api.demo ? 2500 : 60_000);
    const every = setInterval(() => void checkUpdate(api, false), 6 * 3600_000);
    return () => { clearTimeout(first); clearInterval(every); };
  }, [api, on]);
}
