// パスワード変更の共通部分（1件ずつ・まとめての両方で使う）
import { useCallback, useEffect, useRef, useState } from "react";
import { useApp } from "../store";
import type { ApiError, Item } from "../lib/types";
import { changeUrl } from "../lib/api";
import { CopyTimer } from "../components/ui";

export type Phase = "ready" | "busy" | "generated";

export function useChanger(it: Item | undefined) {
  const { api, t, fail, toast, ask } = useApp();
  const [phase, setPhase] = useState<Phase>("ready");
  const [copy, setCopy] = useState<{ what: string | null; stamp: number; seconds: number }>({ what: null, stamp: 0, seconds: 30 });
  const id = it?.id;
  const phaseRef = useRef(phase);
  phaseRef.current = phase;

  useEffect(() => {
    setPhase("ready");
    setCopy({ what: null, stamp: 0, seconds: 30 });
    return () => { if (id) void api.changeEnd(id).catch(() => {}); };
  }, [api, id]);

  const showPasskey = useCallback(async () => {
    await ask({ title: t.passkeyTitle, body: t.passkeyBody, buttons: [{ label: t.ok, kind: "primary", value: "ok" }] });
  }, [ask, t]);

  const generate = useCallback(async () => {
    if (!id) return false;
    setPhase("busy");
    try {
      await api.changeBegin(id);
      await api.changeGenerate(id);
      setPhase("generated");
      return true;
    } catch (e) {
      setPhase("ready");
      fail(e);
      return false;
    }
  }, [api, fail, id]);

  const doCopy = useCallback(async (which: "old" | "new") => {
    if (!id) return;
    try {
      const r = await api.copySecret(id, which);
      setCopy((c) => ({ what: which === "new" ? t.newPw : t.oldPw, stamp: c.stamp + 1, seconds: r.seconds }));
    } catch (e) { fail(e); }
  }, [api, fail, id, t]);

  /** 1Password の中だけ元に戻す。戻せたら true。
   *  1Password の決まりで、書き戻すとその項目のパスキーが消えるため、先に本人に確かめる。 */
  const restore = useCallback(async (quiet = false) => {
    if (!id) return false;
    if (it?.passkey_site) { await showPasskey(); return false; }
    const v = await ask({ title: t.pkAskTitle, body: t.pkAskBody, buttons: [
      { label: t.pkAskUnsure, kind: "ghost", value: "unsure" },
      { label: t.pkAskNo, kind: "primary", value: "no" },
    ] });
    if (v === "unsure") { await showPasskey(); return false; }
    if (v !== "no") return false;
    try {
      await api.changeRestore(id, true);
      setPhase("ready");
      setCopy((c) => ({ ...c, what: null }));
      if (!quiet) toast(t.undone);
      return true;
    } catch (e) {
      if ((e as ApiError)?.code === "passkey_risk") await showPasskey(); else fail(e);
      return false;
    }
  }, [api, ask, fail, id, it, showPasskey, t, toast]);

  const openPage = useCallback(() => {
    if (!it) return;
    const u = changeUrl(it);
    if (u) api.openUrl(u).catch(fail);
  }, [api, fail, it]);

  const finish = useCallback(() => { setPhase("ready"); phaseRef.current = "ready"; setCopy((c) => ({ ...c, what: null })); }, []);
  return { phase, phaseRef, generate, doCopy, restore, openPage, copy, finish };
}

/** 作っている途中・作ったあとに画面を離れようとしたら止める */
export function useLeaveGuard(ch: ReturnType<typeof useChanger>) {
  const { ask, t, toast, leaveGuard } = useApp();
  const ref = useRef(ch);
  ref.current = ch;
  useEffect(() => {
    leaveGuard.current = async () => {
      const c = ref.current;
      if (c.phaseRef.current === "busy") { toast(t.busyWait); return false; }
      if (c.phaseRef.current !== "generated") return true;
      const v = await ask({ title: t.leaveTitle, body: t.leaveBody, buttons: [{ label: t.cancel, kind: "ghost", value: "stay" }, { label: t.leaveKeep, kind: "ghost", value: "keep" }, { label: t.leaveRestore, kind: "primary", value: "restore" }] });
      if (v === "restore") return c.restore(true);
      return v === "keep";
    };
    return () => { leaveGuard.current = null; };
  }, [ask, leaveGuard, t, toast]);
}

export function SiteActions({ ch }: { ch: ReturnType<typeof useChanger> }) {
  const { t } = useApp();
  return (
    <>
      <div style={{ marginBottom: "var(--space-sm)" }}>
        <button type="button" className="btn" onClick={ch.openPage}>{t.openChange}</button>
      </div>
      <div className="secret">
        <div><div className="k">{t.oldPw}</div><span className="v" aria-hidden="true">{t.masked8}</span></div>
        <button type="button" className="btn small" aria-label={t.copyOld} onClick={() => ch.doCopy("old")}>{t.copy}</button>
      </div>
      <div className="secret">
        <div><div className="k">{t.newPw}</div><span className="v" aria-hidden="true">{t.masked8}</span><span className="note">{t.savedNote}</span></div>
        <button type="button" className="btn small" aria-label={t.copyNew} onClick={() => ch.doCopy("new")}>{t.copy}</button>
      </div>
      <CopyTimer what={ch.copy.what} seconds={ch.copy.seconds} stamp={ch.copy.stamp} />
    </>
  );
}
