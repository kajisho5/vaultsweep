import { useEffect, useRef, useState } from "react";
import { useApp } from "../store";
import { Tag, Meter } from "../components/ui";
import { mainKind, type Item } from "../lib/types";
import { SiteActions, useChanger, useLeaveGuard } from "./changeKit";

type St = "todo" | "done" | "skip";

function Current({ it, onDone, onSkip }: { it: Item; onDone(): void; onSkip(): void }) {
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
      if (ch.phaseRef.current === "generated") {
        if (it.passkey_site) {
          // パスキー対応のサイトは自動で戻せないので、新しいパスワードのまま「あとで」にする（元の値はアプリが覚えている）
          toast(t.bSkipKeep);
        } else {
          if (!(await ch.restore(true))) return;
          toast(t.bSkipped);
        }
      }
      onSkip();
    } finally { working.current = false; }
  };
  const keys = useRef({ done, skip });
  keys.current = { done, skip };
  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      // ボタンやリンクに当たっている Enter は、そのボタンの動きを優先する
      const tg = e.target as HTMLElement | null;
      if (tg?.closest("input,select,textarea,button,a,[role=switch],[tabindex],[contenteditable]") || document.querySelector(".modal-bg")) return;
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
        <button type="button" className="btn primary" disabled={ch.phase === "busy"} onClick={async () => { if (await ch.generate()) toast(t.bGenerated); }}>{ch.phase === "busy" ? "…" : t.bGen}</button>
      ) : <SiteActions ch={ch} />}
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
  const skipped = q.filter((s) => s === "skip").length;
  const set = (i: number, s: St) => setQ((old) => old.map((x, j) => (j === i ? s : x)));
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
                  <span className="st">{st === "done" ? "✓" : st === "skip" ? "–" : String(i + 1).padStart(2, "0")}</span>{p.title}<small>{k ? t.kind[k] : ""}</small>
                </li>
              );
            })}
          </ul>
        </div>
        {finished ? (
          <div className="card">
            <h2>{t.bEndTitle}</h2>
            <p className="cap">{t.bEndCap(changed)}</p>
            <div className="okbox"><b>{t.scoreUp(startScore, report?.score ?? startScore)}</b><p>{t.doneNext}</p></div>
            <button type="button" className="btn primary" style={{ marginTop: 8 }} onClick={() => go("issues")}>{t.toList}</button>
          </div>
        ) : (
          <Current key={snap[idx].id} it={item(snap[idx].id) ?? snap[idx]} onDone={() => set(idx, "done")} onSkip={() => set(idx, "skip")} />
        )}
      </div>
    </>
  );
}
