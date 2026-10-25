import { useMemo } from "react";
import { useApp } from "../store";
import { Lock, Meter, SearchIcon, Tag, fmtDate, isBad } from "../components/ui";
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
    <aside className="card detail" aria-label={it.title}>
      <div className="part">
        <h2>{it.title}</h2>
        <div className="mutedc" style={{ fontSize: "var(--text-sm)" }}>{it.domain ?? "—"}</div>
        <dl className="meta">
          <dt>{t.vault}</dt><dd>{it.vault}</dd>
          <dt>{t.username}</dt><dd><span className="m">{it.username_hint ?? "—"}</span>{it.username_hint && ` ${t.masked}`}</dd>
          <dt>{t.lastTouched}</dt><dd><span className="m">{fmtDate(it.updated_at, t.ymd)}</span>{it.days_since_update != null && t.daysAgo(it.days_since_update)}</dd>
          {diff && <><dt>{t.nav.cancel}</dt><dd>{t.deletionInfo(diff)}</dd></>}
        </dl>
      </div>
      <div className="part">
        {it.issues.length === 0 && <p className="cap" style={{ margin: 0 }}>{t.noIssues}</p>}
        {it.issues.map((x) => (
          <div className="issue" key={x.kind}><b className={isBad(x.kind) ? "badc" : ""}>{t.kind[x.kind]}</b><p>{issueText(x, t)}</p></div>
        ))}
      </div>
      <div className="part">
        {it.passkey_site && <p className="hint" style={{ marginBottom: "var(--space-md)" }}>{t.passkeyHint}</p>}
        <div className="stack">
          {pw && <button type="button" className="btn" onClick={() => startChange(it.id)}>{t.action.change}</button>}
          <div className="row2">
            <button type="button" className="btn" disabled={!doc} onClick={() => doc && api.openUrl(doc).catch(fail)}>{t.setTwofa}</button>
            <button type="button" className="btn" disabled={inCancel} onClick={async () => {
              try { setState(await api.cancelAdd(it.id)); toast(t.addedCancel); } catch (e) { fail(e); }
            }}>{inCancel ? t.inCancel : t.toCancel}</button>
          </div>
          {(has(it, "stale") || inUse) && (
            <button type="button" className="btn" onClick={async () => {
              try {
                const r = await api.setInUse(it.id, !inUse);
                if (r) setReport(r);
                setState((await api.bootstrap()).state);
              } catch (e) { fail(e); }
            }}>{inUse ? t.unmarkInUse : t.markInUse}</button>
          )}
          {has(it, "dup") && (
            <button type="button" className="btn coral" onClick={async () => { if (await confirmArchive(it.id, it.title)) setSelId(null); }}>{t.archiveDup}</button>
          )}
        </div>
        <div className="safe"><Lock />{t.safe}</div>
      </div>
    </aside>
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
    <button type="button" key={k ?? "all"} className="chip" aria-pressed={listFilter === k} onClick={() => setListFilter(k)}>
      {label} <b>{n}</b>
    </button>
  );
  return (
    <>
      <h1 className="title">{t.isTitle}</h1>
      <p className="lead">{t.isLead}</p>
      {bulk.length > 0 && (
        <div className="bulkbar"><span>{t.bulk(bulk.length)}</span><button type="button" className="btn primary" onClick={() => startBatch(bulk.map((i) => i.id))}>{t.bulkBtn}</button></div>
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
              {rows.length === 0 && <tr><td colSpan={4} className="mutedc">{t.noRows}</td></tr>}
              {rows.map((p) => (
                <tr key={p.id} className={p.id === sel?.id ? "sel" : ""} onClick={() => setSelId(p.id)} tabIndex={0} onKeyDown={(e) => { if (e.key === "Enter") setSelId(p.id); }}>
                  <td className="site"><b>{p.title}</b><small>{p.domain ?? "—"}</small></td>
                  <td><div className="tags" style={{ marginTop: 0 }}>{p.issues.slice(0, 2).map((x) => <Tag key={x.kind} k={x.kind} />)}{p.issues.length > 2 && <span className="m mutedc" style={{ alignSelf: "center", fontSize: "var(--text-xs)" }}>+{p.issues.length - 2}</span>}</div></td>
                  <td className="hide-sm m">{fmtDate(p.updated_at, t.ymd)}</td>
                  <td><div className={`risk${p.risk >= 0.8 ? " hi" : ""}`}>{Math.round(p.risk * 100)}<Meter value={p.risk} thin /></div></td>
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
