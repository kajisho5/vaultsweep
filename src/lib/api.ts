// 画面から中身を呼ぶ口。アプリの外（ブラウザ）ではデモ用の架空データで動く。
import type { Boot, Item, Pending, Progress, Report, ScanResult, Settings, State } from "./types";
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
  changeGenerate(id: string): Promise<void>;
  copySecret(id: string, which: "old" | "new"): Promise<{ seconds: number }>;
  copyPlain(text: string): Promise<void>;
  changeConfirm(id: string): Promise<Report | null>;
  changeRestore(id: string): Promise<void>;
  changeEnd(id: string): Promise<void>;
  archive(id: string): Promise<State>;
  writeReceipt(id: string, dateLabel: string, date: string, receiptLabel: string, receipt: string): Promise<State>;
  openUrl(url: string): Promise<void>;
  pendingResolve(id: string, approve: boolean): Promise<{ pending: Pending | null; state: State }>;
  exportCsv(header: string[], rows: string[][]): Promise<string>;
  testKeyRef(reference: string): Promise<boolean>;
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
    changeGenerate: (id) => invoke("change_generate", { id }),
    copySecret: (id, which) => invoke("copy_secret", { id, which }),
    copyPlain: (text) => invoke("copy_plain", { text }),
    changeConfirm: (id) => invoke("change_confirm", { id }),
    changeRestore: (id) => invoke("change_restore", { id }),
    changeEnd: (id) => invoke("change_end", { id }),
    archive: (id) => invoke("archive", { id }),
    writeReceipt: (id, dateLabel, date, receiptLabel, receipt) => invoke("write_receipt", { id, dateLabel, date, receiptLabel, receipt }),
    openUrl: (url) => invoke("open_url", { url }),
    pendingResolve: (id, approve) => invoke("pending_resolve", { id, approve }),
    exportCsv: (header, rows) => invoke("export_csv", { header, rows }),
    testKeyRef: (reference) => invoke("test_key_ref", { reference }),
  };
}

let cached: Promise<Api> | null = null;
export function getApi(): Promise<Api> {
  if (!cached) cached = inTauri ? tauriApi() : Promise.resolve(demoApi());
  return cached;
}

export const changeUrl = (it: Item) =>
  it.domain ? `https://${it.domain}/.well-known/change-password` : it.url ?? null;
export const siteUrl = (it: Item) => (it.url && /^https?:\/\//.test(it.url) ? it.url : it.domain ? `https://${it.domain}/` : null);
