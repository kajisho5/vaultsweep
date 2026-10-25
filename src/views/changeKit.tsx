// パスワード変更の共通部分（1件ずつ・まとめての両方で使う）
import { useCallback, useEffect, useRef, useState } from "react";
import { useApp } from "../store";
import type { ApiError, ChangePlan, Item, RuleChars, SiteRule } from "../lib/types";
import { changeUrl } from "../lib/api";
import { CopyTimer, Seg } from "../components/ui";

export type Phase = "ready" | "busy" | "generated";

export function useChanger(it: Item | undefined) {
  const { api, t, fail, toast, ask } = useApp();
  const [phase, setPhase] = useState<Phase>("ready");
  const [copy, setCopy] = useState<{ what: string | null; stamp: number; seconds: number }>({ what: null, stamp: 0, seconds: 30 });
  const id = it?.id;
  const phaseRef = useRef(phase);
  phaseRef.current = phase;
  // サイトの決まりと作り方。決まりを変える欄（開いているか・下書き）
  const [plan, setPlan] = useState<ChangePlan | null>(null);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState<SiteRule>({ length: 24, chars: "all" });
  const loadPlan = useCallback(async () => {
    if (!id) return;
    try {
      const p = await api.changePlan(id);
      setPlan(p);
      setDraft({ length: p.plan.length, chars: p.plan.chars });
      if (p.plan.manual) setEditing(true);
    } catch { setPlan(null); }
  }, [api, id]);
  useEffect(() => { setEditing(false); void loadPlan(); }, [loadPlan]);

  useEffect(() => {
    setPhase("ready");
    setCopy({ what: null, stamp: 0, seconds: 30 });
    // 前に作ったまま確定していなければ、その続きから（元のパスワードはまだ覚えている）
    let live = true;
    if (id) api.changeUnfinished().then((ids) => { if (live && ids.includes(id) && phaseRef.current === "ready") setPhase("generated"); }).catch(() => {});
    return () => { live = false; if (id) void api.changeEnd(id).catch(() => {}); };
  }, [api, id]);

  const showPasskey = useCallback(async () => {
    await ask({ title: t.passkeyTitle, body: t.passkeyBody, buttons: [{ label: t.ok, kind: "primary", value: "ok" }] });
  }, [ask, t]);

  /** 作る（作り直しにも使う）。site を渡すと、このサイトの決まりとして覚えてから作る */
  const generate = useCallback(async (site?: SiteRule | null) => {
    if (!id) return false;
    const again = phaseRef.current === "generated";
    setPhase("busy"); phaseRef.current = "busy";
    try {
      await api.changeBegin(id);
      await api.changeGenerate(id, site ?? null);
      setPhase("generated"); phaseRef.current = "generated";
      if (site) setEditing(false);
      if (again) toast(t.regenerated);
      void loadPlan();
      return true;
    } catch (e) {
      const detail = (e as { detail?: string })?.detail;
      if (detail === "rule_manual") {
        setPhase(again ? "generated" : "ready");
        setEditing(true);
        toast(t.manualNote);
      } else if (detail === "rule_unmet") {
        // 1Password の中は作った値に変わっている。元のパスワードは預かったまま
        setPhase("generated"); phaseRef.current = "generated";
        setEditing(true);
        toast(t.ruleUnmet);
      } else {
        setPhase(again ? "generated" : "ready");
        fail(e);
      }
      void loadPlan();
      return false;
    }
  }, [api, fail, id, loadPlan, t, toast]);

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
    if (v !== "no" || phaseRef.current !== "generated") return false;
    // 戻している間は「保存できた」などを押せないようにする
    setPhase("busy"); phaseRef.current = "busy";
    try {
      await api.changeRestore(id, true);
      setPhase("ready"); phaseRef.current = "ready";
      setCopy((c) => ({ ...c, what: null }));
      if (!quiet) toast(t.undone);
      return true;
    } catch (e) {
      setPhase("generated"); phaseRef.current = "generated";
      if ((e as ApiError)?.code === "passkey_risk") await showPasskey(); else fail(e);
      return false;
    }
  }, [api, ask, fail, id, it, showPasskey, t, toast]);

  const openPage = useCallback(() => {
    if (!it) return;
    const u = changeUrl(it);
    if (u) api.openUrl(u).catch(fail); else toast(t.noUrl);
  }, [api, fail, it, t, toast]);

  const finish = useCallback(() => { setPhase("ready"); phaseRef.current = "ready"; setCopy((c) => ({ ...c, what: null })); }, []);
  const clearRule = useCallback(async () => {
    if (!plan?.domain) return;
    try { await api.siteRuleClear(plan.domain); toast(t.ruleResetDone); setEditing(false); await loadPlan(); } catch (e) { fail(e); }
  }, [api, fail, loadPlan, plan, t, toast]);
  return { phase, phaseRef, generate, doCopy, restore, openPage, copy, finish, plan, editing, setEditing, draft, setDraft, clearRule };
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
        <div><div className="k">{t.newPw}</div><span className="v" aria-hidden="true">{t.masked8}</span><span className="saved-note">{t.savedNote}</span></div>
        <button type="button" className="btn small" aria-label={t.copyNew} onClick={() => ch.doCopy("new")}>{t.copy}</button>
      </div>
      <CopyTimer what={ch.copy.what} seconds={ch.copy.seconds} stamp={ch.copy.stamp} />
    </>
  );
}

/** 決まりを変える欄（長さ・使う文字） */
function RuleEditor({ ch }: { ch: ReturnType<typeof useChanger> }) {
  const { t } = useApp();
  const d = ch.draft;
  return (
    <div className="rulebox" style={{ marginTop: "var(--space-sm)" }}>
      <div className="set-row">{t.ruleLenLabel}
        <input className="inp m" type="number" min={4} max={64} style={{ width: 88 }} aria-label={t.ruleLenLabel} value={d.length}
          onChange={(e) => { const v = Number(e.target.value); if (Number.isFinite(v)) ch.setDraft({ ...d, length: v }); }}
          onBlur={() => ch.setDraft({ ...d, length: Math.min(64, Math.max(4, Math.round(d.length) || 4)) })} />
      </div>
      <div className="set-row">{t.ruleCharsLabel}
        <Seg<RuleChars> label={t.ruleCharsLabel} value={d.chars} onChange={(v) => ch.setDraft({ ...d, chars: v })}
          options={[["all", t.charsV.all], ["alnum", t.charsV.alnum], ["digits", t.charsV.digits]]} />
      </div>
      <p className="cap" style={{ margin: "var(--space-xs) 0 0" }}>{t.ruleRange}</p>
    </div>
  );
}

const clampRule = (r: SiteRule): SiteRule => ({ length: Math.min(64, Math.max(4, Math.round(r.length) || 4)), chars: r.chars });

/** 手順1：このサイトの決まりと、作り方。決まりを変える欄 */
export function RuleBox({ ch }: { ch: ReturnType<typeof useChanger> }) {
  const { t } = useApp();
  const p = ch.plan;
  if (!p) return null;
  const r = p.rule;
  const rows: [string, string][] = [];
  if (r) {
    if (r.min != null || r.max != null) rows.push([t.ruleLen, t.ruleLenV(r.min, r.max)]);
    if (r.symbols != null) rows.push([t.ruleSym, t.ruleSymV(r.symbols)]);
    if (r.required.length) rows.push([t.ruleReq, r.required.map((k) => t.ruleReqV[k]).join(" · ")]);
    if (r.max_consecutive) rows.push([t.ruleRun, t.ruleRunV(r.max_consecutive)]);
  }
  const showOpNote = !!r && r.symbols != null && p.plan.source === "rule" && p.plan.chars !== "all" && !p.plan.manual;
  return (
    <div style={{ marginBottom: "var(--space-sm)" }}>
      <div className="rulebox">
        <div className="rulehead"><b>{t.ruleTitle}</b>{t.ruleSrc[p.plan.source] && <small className={p.plan.source === "rule" ? "cool" : ""}>{t.ruleSrc[p.plan.source]}</small>}</div>
        {p.plan.source === "site" && p.site ? null : rows.length ? (
          <dl className="rulelist">{rows.map(([a, b]) => <div key={a}><dt>{a}</dt><dd className={a === t.ruleSym ? "m" : ""}>{b}</dd></div>)}</dl>
        ) : <p className="cap" style={{ margin: 0 }}>{t.ruleNone}</p>}
        {p.plan.manual ? <p className="warn" style={{ margin: "var(--space-xs) 0 0" }}>{t.manualNote}</p>
          : <p className="ruleplan">{t.planLine(p.plan.length, t.charsV[p.plan.chars])}</p>}
      </div>
      {showOpNote && <p className="cap" style={{ marginTop: "var(--space-xs)" }}>{t.opSymNote}</p>}
      <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap", marginTop: "var(--space-xs)" }}>
        <button type="button" className="btn small" aria-expanded={ch.editing} onClick={() => ch.setEditing(!ch.editing)}>{ch.editing ? t.ruleEditClose : t.ruleEdit}</button>
        <small className="mutedc" style={{ fontSize: "var(--text-xs)" }}>{t.ruleEditNote}</small>
        {p.site && p.domain && <button type="button" className="btn small" onClick={() => void ch.clearRule()}>{t.ruleReset}</button>}
      </div>
      {ch.editing && <RuleEditor ch={ch} />}
    </div>
  );
}

/** 手順1の「作って保存」：決まりを変える欄が開いていれば、その決まりで作る */
export function genArg(ch: ReturnType<typeof useChanger>): SiteRule | null {
  return ch.editing ? clampRule(ch.draft) : null;
}

/** 手順2：サイトに断られたときの作り直し */
export function RejectBox({ ch }: { ch: ReturnType<typeof useChanger> }) {
  const { t } = useApp();
  const p = ch.plan?.plan;
  const len = p?.length ?? 24;
  const chars = p?.chars ?? "all";
  const busy = ch.phase === "busy";
  const opts: [string, SiteRule][] = [
    [t.retryNoSym, { length: len, chars: "alnum" }],
    [t.retry16, { length: 16, chars: chars === "digits" ? "alnum" : chars }],
    [t.retryDigits, { length: Math.min(len, 8), chars: "digits" }],
  ];
  return (
    <div className="rulebox" style={{ marginTop: "var(--space-md)" }}>
      <div className="rulehead"><b>{t.rejectTitle}</b></div>
      <p className="cap" style={{ margin: "0 0 var(--space-sm)" }}>{t.rejectCap}</p>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        {opts.map(([l, r]) => <button key={l} type="button" className="btn small" disabled={busy} aria-busy={busy} onClick={() => void ch.generate(r)}>{l}</button>)}
      </div>
      <div className="set-row" style={{ borderBottom: 0, marginTop: "var(--space-xs)" }}>{t.rejectMore}
        <button type="button" className="btn small" aria-expanded={ch.editing} onClick={() => ch.setEditing(!ch.editing)}>{ch.editing ? t.ruleEditClose : t.ruleEdit}</button>
      </div>
      {ch.editing && <>
        <RuleEditor ch={ch} />
        <button type="button" className="btn" style={{ marginTop: "var(--space-sm)" }} disabled={busy} onClick={() => void ch.generate(clampRule(ch.draft))}>{busy ? t.working : t.regenWith}</button>
      </>}
      <p className="cap" style={{ margin: "var(--space-xs) 0 0" }}>{t.rejectRemember}</p>
    </div>
  );
}
