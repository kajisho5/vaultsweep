import { useEffect, useState } from "react";
import { useApp } from "../store";
import { CountUp, Spark, Tag, fmtDate, kc } from "../components/ui";
import { KINDS, PASSWORD_KINDS, mainKind, type Item, type Pending } from "../lib/types";
import { issueText } from "../lib/i18n";
import { siteUrl } from "../lib/api";

export function PendingBar() {
  const { state, t, api, setState, setReport, toast, fail, startChange, item } = useApp();
  if (!state.pending.length) return null;
  const resolve = async (p: Pending, approve: boolean) => {
    try {
      if (approve && p.kind === "change") {
        const r = await api.pendingResolve(p.id, false);
        setState(r.state);
        if (item(p.item_id)) startChange(p.item_id);
        return;
      }
      const r = await api.pendingResolve(p.id, approve);
      setState(r.state);
      if (approve) { setReport((await api.bootstrap()).report); toast(t.archived); }
    } catch (e) { fail(e); }
  };
  return (
    <div style={{ marginBottom: 16 }}>
      {state.pending.map((p) => (
        <div className="pending" key={p.id}>
          <span><b>{t.pendingTitle}</b>：{p.kind === "archive" ? t.pendingArchive(p.title) : t.pendingChange(p.title)}</span>
          <span style={{ display: "flex", gap: 8 }}>
            <button className="btn ghost small" onClick={() => resolve(p, false)}>{t.deny}</button>
            <button className={`btn small ${p.kind === "archive" ? "coral" : "primary"}`} onClick={() => resolve(p, true)}>{t.approve}</button>
          </span>
        </div>
      ))}
    </div>
  );
}

function FirstRun() {
  const { t, scan, scanning } = useApp();
  return (
    <>
      <h1 className="title">{t.firstTitle}</h1>
      <p className="lead">{t.firstLead}</p>
      <div className="card" style={{ maxWidth: 640 }}>
        <ol className="how">{t.firstSteps.map((s) => <li key={s}>{s}</li>)}</ol>
        <button className="btn coral" onClick={() => void scan()} disabled={scanning}>{scanning ? t.scanning : t.scan}</button>
      </div>
    </>
  );
}

export function useAct() {
  const { t, startChange, go, api, setListFilter, setSelId, toast, fail } = useApp();
  return (it: Item) => {
    const k = mainKind(it);
    if (!k) return { label: "", run: () => {} };
    if (PASSWORD_KINDS.includes(k)) return { label: t.action.change, run: () => startChange(it.id) };
    if (k === "nofa") return { label: t.action.twofa, run: () => { const u = it.twofa_doc ?? siteUrl(it); if (u) api.openUrl(u).catch(fail); else toast(t.err.other); } };
    if (k === "stale") return { label: t.action.stale, run: () => { setListFilter("stale"); setSelId(it.id); void go("issues"); } };
    return { label: t.action.dup, run: () => { setListFilter("dup"); setSelId(it.id); void go("issues"); } };
  };
}

export function Overview() {
  const { t, report, state, nextRun, item, todoIds, go, setListFilter, setQuery } = useApp();
  const act = useAct();
  const [shown, setShown] = useState(false);
  useEffect(() => { const id = requestAnimationFrame(() => setShown(true)); return () => cancelAnimationFrame(id); }, []);
  if (!report) return <><PendingBar /><FirstRun /></>;

  const total = report.total;
  const flaggedItems = report.items.filter((i) => i.issues.length);
  const flagged = flaggedItems.length;
  const clean = total - flagged;
  const pct = total ? Math.floor((clean / total) * 100) : 100;
  const goal = Math.min(100, Math.floor(pct / 5) * 5 + 5);
  const more = Math.max(1, Math.ceil((total * goal) / 100) - clean);
  const hist = state.history;
  const prev = hist.length >= 2 ? hist[hist.length - 2].score : null;
  const delta = prev == null ? null : report.score - prev;
  const lastNew = hist.length ? hist[hist.length - 1].new_issues : 0;
  const week = Date.now() - 7 * 86400000;
  const fixedWeek = state.fixed.filter((f) => new Date(f.at).getTime() >= week).length;
  let streak = 0;
  for (let w = 0; w < 52; w++) {
    const a = Date.now() - (w + 1) * 7 * 86400000, b = Date.now() - w * 7 * 86400000;
    if (state.fixed.some((f) => { const x = new Date(f.at).getTime(); return x >= a && x < b; })) streak++; else break;
  }
  const cancel = Object.values(state.cancel);
  const cancelOpen = cancel.filter((c) => c.state !== "archived").length;
  const cancelLeft = cancel.filter((c) => c.state === "left").length;
  const max = Math.max(1, ...KINDS.map((k) => report.counts[k] ?? 0));
  const byMain = (k: (typeof KINDS)[number]) => flaggedItems.filter((i) => mainKind(i) === k).length;
  const todos = todoIds.map((e) => ({ ...e, it: item(e.id) }));

  return (
    <>
      <PendingBar />
      <h1 className="title">{t.ovTitle(total, flagged)}</h1>
      <p className="lead">{t.ovLead(fmtDate(report.scanned_at, t.dateTime), nextRun ? t.dateTime(new Date(nextRun)) : null)}</p>
      {report.warnings.includes("offline") && <div className="warn">{t.warnOffline}</div>}
      {report.warnings.includes("hibp_failed") && <div className="warn">{t.warnHibp}</div>}
      <div className="kpis">
        <div className="kpi"><span>{t.k1}</span><strong><CountUp value={report.score} /></strong><small>{t.k1d(delta)}</small></div>
        <div className="kpi"><span>{t.k2}</span><strong><CountUp value={flagged} /></strong><small>{t.k2d(lastNew)}</small></div>
        <div className="kpi"><span>{t.k4}</span><strong><CountUp value={fixedWeek} /></strong><small>{t.k4d(streak)}</small></div>
        <div className="kpi"><span>{t.k3}</span><strong><CountUp value={cancelOpen} /></strong><small>{t.k3d(cancelLeft)}</small></div>
      </div>
      <div className="card" style={{ marginBottom: 16 }}>
        <div className="prog-head"><h2>{t.progTitle}</h2><span>{pct}%</span></div>
        <div className="prog-bar" role="img" aria-label={`${pct}%`}>
          <i className="clean" style={{ width: shown ? `${(clean / Math.max(1, total)) * 100}%` : 0 }} />
          {KINDS.map((k) => <i key={k} className="seg" style={{ ...kc(k), width: shown ? `${(byMain(k) / Math.max(1, total)) * 100}%` : 0 }} />)}
        </div>
        <p className="cap" style={{ margin: "10px 0 0" }}>{flagged ? t.progMsg(clean, total, more, goal) : t.progAll}</p>
      </div>
      <div className="grid2" style={{ gridTemplateColumns: "minmax(0,1.1fr) minmax(320px,1fr)" }}>
        <div className="card">
          <h2>{t.todoTitle}</h2>
          <p className="cap">{t.todoCap}</p>
          <div className="todo">
            {todos.length === 0 && <p className="cap">{t.todoEmpty}</p>}
            {todos.map(({ id, kind, it }, i) => {
              const fixed = !it || !it.issues.some((x) => x.kind === kind);
              const k = it ? mainKind(it) : null;
              const a = it ? act(it) : null;
              return (
                <div key={id} className={`todo-item${fixed ? " is-fixed" : ""}`} style={{ animationDelay: `${i * 120}ms` }}>
                  <div className="num">{fixed ? "✓" : i + 1}</div>
                  <div>
                    <h3>{it?.title ?? "—"}</h3>
                    {it && k && !fixed && <p>{issueText(it.issues.find((x) => x.kind === k)!, t)}</p>}
                    {it && <div className="tags">{it.issues.map((x) => <Tag key={x.kind} k={x.kind} />)}</div>}
                  </div>
                  {fixed || !a ? <span className="done-label" style={{ color: "var(--ok)", fontWeight: 700, fontSize: 13 }}>{t.todoDone}</span>
                    : <button className={`btn ${i === 0 ? "coral" : "primary"}`} onClick={a.run}>{a.label}</button>}
                </div>
              );
            })}
          </div>
        </div>
        <div>
          <div className="card" style={{ marginBottom: 16 }}>
            <h2>{t.brTitle}</h2>
            <p className="cap">{t.brCap}</p>
            <div className="brk">
              {KINDS.map((k) => (
                <button key={k} style={kc(k)} onClick={() => { setListFilter(k); setQuery(""); void go("issues"); }}>
                  <i /><span>{t.kind[k]}<small>{t.kindDesc[k]}</small></span><b>{report.counts[k] ?? 0}</b>
                  <span className="mini"><span style={{ width: `${((report.counts[k] ?? 0) / max) * 100}%` }} /></span><span className="chev">›</span>
                </button>
              ))}
            </div>
          </div>
          <div className="card">
            <h2>{t.trendTitle}</h2>
            <p className="cap">{hist.length >= 2 ? t.trendCap : t.trendFew}</p>
            <Spark values={hist.slice(-12).map((h) => h.score)} label={t.trendTitle} />
          </div>
          {report.email_breaches.length > 0 && (
            <div className="card" style={{ marginTop: 16 }}>
              <h2>{t.emailTitle}</h2>
              {report.email_breaches.map((e) => <div className="src" key={e.masked}>{e.masked}<span>{e.breaches.join(", ")}</span></div>)}
            </div>
          )}
        </div>
      </div>
    </>
  );
}

