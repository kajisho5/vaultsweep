import { useMemo } from "react";
import { useApp } from "../store";
import { Lock, SearchIcon, Tag, fmtDate, kc, riskColor } from "../components/ui";
import { KINDS, PASSWORD_KINDS, has, type Item } from "../lib/types";
import { issueText } from "../lib/i18n";
import { siteUrl } from "../lib/api";

function Detail({ it }: { it: Item }) {
  const { t, startChange, api, state, setState, setReport, toast, fail, confirmArchive, setSelId } = useApp();
  const inCancel = !!state.cancel[it.id] && state.cancel[it.id].state !== "archived";
  const inUse = state.in_use.includes(it.id);
  const pw = it.issues.some((x) => PASSWORD_KINDS.includes(x.kind));
  const doc = it.twofa_doc ?? (has(it, "nofa") ? siteUrl(it) : null);
  const diff = it.deletion ? t.difficulty[it.deletion.difficulty] ?? it.deletion.difficulty : null;
  return (
    <div className="card detail">
      <h2>{it.title}</h2>
      <div style={{ color: "var(--ink2)", fontSize: 13 }}>{it.domain ?? "—"}</div>
      <div className="meta">
        <span>{t.vault}</span><b>{it.vault}</b>
        <span>{t.username}</span><b className="mono">{it.username_hint ? `${it.username_hint} ${t.masked}` : "—"}</b>
        <span>{t.lastTouched}</span><b>{fmtDate(it.updated_at, t.ymd)}{it.days_since_update != null && t.daysAgo(it.days_since_update)}</b>
        {diff && <><span>{t.nav.cancel}</span><b>{t.deletionInfo(diff)}</b></>}
      </div>
      {it.issues.length === 0 && <p className="cap">{t.noIssues}</p>}
      {it.issues.map((x) => (
        <div className="issue" key={x.kind} style={kc(x.kind)}><b>{t.kind[x.kind]}</b><p>{issueText(x, t)}</p></div>
      ))}
      {it.passkey_site && <div className="hint">{t.passkeyHint}</div>}
      <div className="stack">
        {pw && <button className="btn coral" onClick={() => startChange(it.id)}>{t.action.change}</button>}
        <div className="row2">
          <button className="btn ghost" disabled={!doc} onClick={() => doc && api.openUrl(doc).catch(fail)}>{t.setTwofa}</button>
          <button className="btn ghost" disabled={inCancel} onClick={async () => {
            try { setState(await api.cancelAdd(it.id)); toast(t.addedCancel); } catch (e) { fail(e); }
          }}>{inCancel ? t.inCancel : t.toCancel}</button>
        </div>
        {(has(it, "stale") || inUse) && (
          <button className="btn ghost" onClick={async () => {
            try {
              const r = await api.setInUse(it.id, !inUse);
              if (r) setReport(r);
              setState((await api.bootstrap()).state);
            } catch (e) { fail(e); }
          }}>{inUse ? t.unmarkInUse : t.markInUse}</button>
        )}
        {has(it, "dup") && (
          <button className="btn ghost" onClick={async () => { if (await confirmArchive(it.id, it.title)) setSelId(null); }}>{t.archiveDup}</button>
        )}
      </div>
      <div className="safe"><Lock />{t.safe}</div>
    </div>
  );
}

export function Issues() {
  const { t, report, listFilter, setListFilter, query, setQuery, selId, setSelId, startBatch } = useApp();
  const flagged = useMemo(() => report?.items.filter((i) => i.issues.length) ?? [], [report]);
  const q = query.trim().toLowerCase();
  const rows = useMemo(() => {
    const base = q ? report?.items ?? [] : flagged;
    return base.filter((i) => (!listFilter || has(i, listFilter)) && (!q || i.title.toLowerCase().includes(q) || (i.domain ?? "").includes(q) || (i.url ?? "").toLowerCase().includes(q)));
  }, [flagged, report, listFilter, q]);
  const bulk = flagged.filter((i) => i.issues.some((x) => x.kind === "leak" || x.kind === "breach" || x.kind === "reuse" || x.kind === "similar"));
  if (!report) return <><h1 className="title">{t.isTitle}</h1><p className="lead">{t.firstTitle}</p></>;
  const sel = rows.find((r) => r.id === selId) ?? rows[0];
  const chip = (k: (typeof KINDS)[number] | null, label: string, n: number) => (
    <button key={k ?? "all"} className="chip" style={k ? kc(k) : undefined} aria-pressed={listFilter === k} onClick={() => setListFilter(k)}>
      {k && <i />}{label} <b>{n}</b>
    </button>
  );
  return (
    <>
      <h1 className="title">{t.isTitle}</h1>
      <p className="lead">{t.isLead}</p>
      {bulk.length > 0 && (
        <div className="bulkbar"><span>{t.bulk(bulk.length)}</span><button className="btn primary" onClick={() => startBatch(bulk.map((i) => i.id))}>{t.bulkBtn}</button></div>
      )}
      <div className="filters">
        <label className="search"><SearchIcon /><input value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t.searchSite} aria-label={t.searchSite} /></label>
        {chip(null, t.all, flagged.length)}
        {KINDS.map((k) => chip(k, t.kind[k], report.counts[k] ?? 0))}
      </div>
      <div className="grid2 keep2" style={{ gridTemplateColumns: "minmax(0,1.6fr) minmax(320px,1fr)" }}>
        <div className="tbl-wrap">
          <table className="tbl click">
            <thead><tr><th>{t.thSite}</th><th>{t.thIssues}</th><th className="hide-sm">{t.thUpdated}</th><th>{t.thRisk}</th></tr></thead>
            <tbody>
              {rows.length === 0 && <tr><td colSpan={4} style={{ color: "var(--ink3)" }}>{t.noRows}</td></tr>}
              {rows.map((p) => (
                <tr key={p.id} className={p.id === sel?.id ? "sel" : ""} onClick={() => setSelId(p.id)} tabIndex={0} onKeyDown={(e) => { if (e.key === "Enter") setSelId(p.id); }}>
                  <td className="site"><b>{p.title}</b><small>{p.domain ?? "—"}</small></td>
                  <td><div className="tags">{p.issues.map((x) => <Tag key={x.kind} k={x.kind} />)}</div></td>
                  <td className="hide-sm">{fmtDate(p.updated_at, t.ymd)}</td>
                  <td><div className="risk"><i style={{ ["--w" as string]: `${Math.round(p.risk * 100)}%`, ["--c" as string]: riskColor(p.risk) }} />{Math.round(p.risk * 100)}</div></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {sel ? <Detail it={sel} /> : <div />}
      </div>
    </>
  );
}
