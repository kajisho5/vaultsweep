// 画面から中身を呼ぶ口。アプリの外（ブラウザ）ではデモ用の架空データで動く。
import type { Boot, ChangePlan, Item, Pending, Progress, Report, ScanResult, Settings, SiteRule, State } from "./types";
import { demoApi } from "./demo";

export interface Saved { settings: Settings; next_run: string | null; schedule_error: string | null }

export interface Api {
  demo: boolean;
  bootstrap(): Promise<Boot>;
  checkConnection(): Promise<{ version: string; vaults: number; account: string | null }>;
  runScan(onProgress: (p: Progress) => void): Promise<ScanResult>;
  saveSettings(s: Settings): Promise<Saved>;
  setInUse(id: string, on: boolean): Promise<Report | null>;
  cancelAdd(id: string): Promise<State>;
  cancelSet(id: string, state: "todo" | "left" | "archived" | "remove", receipt?: string | null): Promise<State>;
  changeBegin(id: string): Promise<void>;
  /** site を渡すと、このサイトの決まりとして覚えてから作る */
  changeGenerate(id: string, site?: SiteRule | null): Promise<void>;
  /** このサイトの決まりと、使う作り方 */
  changePlan(id: string): Promise<ChangePlan>;
  siteRuleClear(domain: string): Promise<void>;
  copySecret(id: string, which: "old" | "new"): Promise<{ seconds: number }>;
  copyPlain(text: string): Promise<void>;
  changeConfirm(id: string): Promise<Report | null>;
  changeRestore(id: string, noPasskey: boolean): Promise<void>;
  changeEnd(id: string): Promise<void>;
  /** 新しいパスワードを作ったまま、サイトでの変更を確定していない項目 */
  changeUnfinished(): Promise<string[]>;
  /** 確定していない項目があっても閉じる（本人が選んだとき） */
  quitNow(): Promise<void>;
  /** ウィンドウを閉じようとしたが、確定していない項目があって止めたとき */
  onCloseGuard(cb: (ids: string[]) => void): Promise<() => void>;
  archive(id: string): Promise<State>;
  writeReceipt(id: string, dateLabel: string, date: string, receiptLabel: string, receipt: string): Promise<State>;
  openUrl(url: string): Promise<void>;
  pendingResolve(id: string, approve: boolean): Promise<{ pending: Pending | null; state: State }>;
  exportCsv(header: string[], rows: string[][]): Promise<string>;
  testKeyRef(reference: string): Promise<boolean>;
  /** 新しい版があるか。なければ null */
  updateCheck(): Promise<{ version: string; current: string } | null>;
  /** 新しい版を入れて開き直す（進み具合 0〜100、分からなければ null） */
  updateInstall(onProgress: (pct: number | null) => void): Promise<void>;
}

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriApi(): Promise<Api> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  return {
    demo: false,
    bootstrap: () => invoke("bootstrap"),
    checkConnection: () => invoke("check_connection"),
    async runScan(onProgress) {
      const un = await listen<Progress>("scan-progress", (e) => onProgress(e.payload));
      try {
        return await invoke<ScanResult>("run_scan");
      } finally {
        un();
      }
    },
    saveSettings: (settings) => invoke("save_settings", { settings }),
    setInUse: (id, on) => invoke("set_in_use", { id, on }),
    cancelAdd: (id) => invoke("cancel_add", { id }),
    cancelSet: (id, state, receipt = null) => invoke("cancel_set", { id, state, receipt }),
    changeBegin: (id) => invoke("change_begin", { id }),
    changeGenerate: (id, site = null) => invoke("change_generate", { id, site }),
    changePlan: (id) => invoke("change_plan", { id }),
    siteRuleClear: (domain) => invoke("site_rule_clear", { domain }),
    copySecret: (id, which) => invoke("copy_secret", { id, which }),
    copyPlain: (text) => invoke("copy_plain", { text }),
    changeConfirm: (id) => invoke("change_confirm", { id }),
    changeRestore: (id, noPasskey) => invoke("change_restore", { id, noPasskey }),
    changeEnd: (id) => invoke("change_end", { id }),
    changeUnfinished: () => invoke("change_unfinished"),
    quitNow: () => invoke("quit_now"),
    onCloseGuard: (cb) => listen<string[]>("close-guard", (e) => cb(e.payload)),
    archive: (id) => invoke("archive", { id }),
    writeReceipt: (id, dateLabel, date, receiptLabel, receipt) => invoke("write_receipt", { id, dateLabel, date, receiptLabel, receipt }),
    openUrl: (url) => invoke("open_url", { url }),
    pendingResolve: (id, approve) => invoke("pending_resolve", { id, approve }),
    exportCsv: (header, rows) => invoke("export_csv", { header, rows }),
    testKeyRef: (reference) => invoke("test_key_ref", { reference }),
    updateCheck: () => invoke("update_check"),
    async updateInstall(onProgress) {
      const un = await listen<number | null>("update-progress", (e) => onProgress(e.payload));
      try { await invoke("update_install"); } finally { un(); }
    },
  };
}

let cached: Promise<Api> | null = null;
export function getApi(): Promise<Api> {
  if (!cached) cached = inTauri ? tauriApi() : Promise.resolve(demoApi());
  return cached;
}

export const changeUrl = (it: Item) =>
  it.domain ? `https://${it.domain}/.well-known/change-password` : it.url && /^https:\/\//.test(it.url) ? it.url : null;
export const siteUrl = (it: Item) => (it.url && /^https?:\/\//.test(it.url) ? it.url : it.domain ? `https://${it.domain}/` : null);
