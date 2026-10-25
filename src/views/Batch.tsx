import { useEffect, useRef, useState } from "react";
import { useApp } from "../store";
import { Tag, Meter } from "../components/ui";
import { PASSWORD_KINDS, mainKind, type Item } from "../lib/types";
import { RejectBox, RuleBox, SiteActions, genArg, useChanger, useLeaveGuard } from "./changeKit";

/** done=変えた skip=あとで kept=新しいパスワードのまま「あとで」 gone=順番が来たときには片付いていた */
type St = "todo" | "done" | "skip" | "kept" | "gone";

function Current({ it, onDone, onSkip }: { it: Item; onDone(): void; onSkip(kept: boolean): void }) {
  const { t, api, setReport, setState, fail, toast } = useApp();
  const working = useRef(false);
  const ch = useChanger(it);
  const gen = ch.phase === "generated";

  const done = async () => {
    if (!gen || working.current) return;
    working.current = true;
    try {
      const r = await api.changeConfirm(it.id);
      if (r) setReport(r);
      setState((await api.bootstrap()).state);
      ch.finish();
      onDone();
    } catch (e) { fail(e); } finally { working.current = false; }
  };
  const skip = async () => {
    if (ch.phaseRef.current === "busy" || working.current) return;
    working.current = true;
    try {
      let kept = false;
      if (ch.phaseRef.current === "generated") {
        if (it.passkey_site) {
          kept = true;
          // パスキー対応のサイトは自動で戻せないので、新しいパスワードのまま「あとで」にする（元の値はアプリが覚えている）
          toast(t.bSkipKeep);
        } else {
          if (!(await ch.restore(true))) return;
          toast(t.bSkipped);
        }
      }
      onSkip(kept);
    } finally { working.current = false; }
  };
  const keys = useRef({ done, skip });
  keys.current = { done, skip };
  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      // ボタンやリンクに当たっている Enter は、そのボタンの動きを優先する
      const tg = e.target as HTMLElement | null;
      if (tg?.closest("input,select,textarea,button,a,[role=switch],[tabindex]:not([tabindex=\"-1\"]),[contenteditable]") || document.querySelector(".modal-bg")) return;
      if (e.ctrlKey || e.metaKey || e.altKey || e.repeat || e.isComposing) return;
      if (e.key === "Enter") { e.preventDefault(); void keys.current.done(); }
      if (e.key === "s" || e.key === "S") void keys.current.skip();
    };
    addEventListener("keydown", on);
    return () => removeEventListener("keydown", on);
  }, []);
  useLeaveGuard(ch);

  return (
    <div className="card">
      <div style={{ display: "flex", justifyContent: "space-between", gap: 10, flexWrap: "wrap" }}>
        <div><h2>{it.title}</h2><div className="mutedc" style={{ fontSize: "var(--text-sm)" }}>{it.domain}</div></div>
        <div className="tags">{it.issues.map((x) => <Tag key={x.kind} k={x.kind} />)}</div>
      </div>
      <ol className="how">
        <li style={gen ? { color: "var(--color-ok)" } : undefined}>{t.bStep1} {gen ? "✓" : ""}</li>
        <li>{t.bStep2}</li>
      </ol>
      {it.passkey_site && !gen && <div className="warn">{t.passkeyWarn}</div>}
      {!gen ? (
        <>
          <RuleBox ch={ch} />
          <button type="button" className="btn primary" disabled={ch.phase === "busy" || (!!ch.plan?.plan.manual && !ch.editing)} aria-busy={ch.phase === "busy"} onClick={async () => { if (await ch.generate(genArg(ch))) toast(t.bGenerated); }}>{ch.phase === "busy" ? t.working : t.bGen}</button>
        </>
      ) : <><SiteActions ch={ch} /><RejectBox ch={ch} /></>}
      <div className="actions">
        <button type="button" className={`btn${gen ? " primary" : ""}`} disabled={!gen} onClick={done}>{t.bNext}</button>
        <button type="button" className="btn" disabled={ch.phase === "busy"} onClick={skip}>{t.bSkip}</button>
        {gen && <button type="button" className="btn" onClick={() => void ch.restore()}>{t.bUndo}</button>}
      </div>
      <p className="cap" style={{ marginTop: 12 }}>{t.bKeys}</p>
    </div>
  );
}

export function Batch() {
  const { t, batchIds, item, go, report, celebrate } = useApp();
  const [snap] = useState(() => batchIds.map((id) => item(id)).filter((x): x is Item => !!x));
  const [q, setQ] = useState<St[]>(() => snap.map(() => "todo"));
  const [startScore] = useState(() => report?.score ?? 0);
  const idx = q.findIndex((s) => s === "todo");
  const finished = idx < 0;
  const nDone = q.filter((s) => s !== "todo").length;
  const changed = q.filter((s) => s === "done").length;
  const skipped = q.filter((s) => s === "skip" || s === "kept").length;
  const kept = snap.filter((_, i) => q[i] === "kept");
  const set = (i: number, s: St) => setQ((old) => old.map((x, j) => (j === i ? s : x)));
  // 使い回し・似ているの相手を先に直すと、順番が来る前に片付くことがある。そのときは作り直させずに飛ばす
  const cur = idx >= 0 ? item(snap[idx].id) : undefined;
  const curGone = idx >= 0 && (!cur || !cur.issues.some((x) => PASSWORD_KINDS.includes(x.kind)));
  useEffect(() => { if (curGone) set(idx, "gone"); }, [curGone, idx]);
  useEffect(() => { if (finished && changed > 0) celebrate(); }, [finished]); // eslint-disable-line react-hooks/exhaustive-deps
  return (
    <>
      <button type="button" className="btn" onClick={() => go("issues")} style={{ marginBottom: "var(--space-md)" }}>{t.back}</button>
      <h1 className="title">{finished ? t.bTitleDone(snap.length) : t.bTitle(snap.length)}</h1>
      <p className="lead">{t.bLead}</p>
      <div className="grid2" style={{ gridTemplateColumns: "minmax(280px,.8fr) minmax(0,1.4fr)" }}>
        <div className="card">
          <h2>{t.bQueue}</h2>
          <p className="cap">{t.bProg(nDone, snap.length, changed, skipped)}</p>
          <Meter value={snap.length ? nDone / snap.length : 0} tone="cool" />
          <ul className="queue">
            {snap.map((p, i) => {
              const st = q[i];
              const k = mainKind(p);
              return (
                <li key={p.id} className={st === "todo" ? (i === idx ? "now" : "") : st}>
                  <span className="st">{st === "done" || st === "gone" ? "✓" : st === "skip" ? "–" : st === "kept" ? "!" : String(i + 1).padStart(2, "0")}</span>{p.title}<small className={st === "kept" ? "badc" : undefined}>{st === "gone" ? t.bGone : st === "kept" ? t.bKept : k ? t.kind[k] : ""}</small>
                </li>
              );
            })}
          </ul>
        </div>
        {finished ? (
          <div className="card">
            <h2>{changed ? t.bEndTitle : t.bEndNoneTitle}</h2>
            <p className="cap">{changed ? t.bEndCap(changed) : t.bEndNone}</p>
            {kept.length > 0 && <div className="warn">{t.bKeptWarn(kept.map((p) => p.title).join(t.dot))}</div>}
            {changed > 0 && <div className="okbox"><b>{t.scoreUp(startScore, report?.score ?? startScore)}</b><p>{t.doneNext}</p></div>}
            <button type="button" className="btn primary" style={{ marginTop: 8 }} onClick={() => go("issues")}>{t.toList}</button>
          </div>
        ) : curGone ? <div className="card" /> : (
          <Current key={snap[idx].id} it={cur ?? snap[idx]} onDone={() => set(idx, "done")} onSkip={(k) => set(idx, k ? "kept" : "skip")} />
        )}
      </div>
    </>
  );
}
