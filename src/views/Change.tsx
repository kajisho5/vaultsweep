import { useState } from "react";
import { useApp } from "../store";
import { Tag } from "../components/ui";
import { has } from "../lib/types";
import { siteUrl } from "../lib/api";
import { SiteActions, useChanger, useLeaveGuard } from "./changeKit";

export function Change() {
  const { t, changeId, item, go, api, report, setReport, setState, settings, celebrate, fail } = useApp();
  const [saving, setSaving] = useState(false);
  const [snapshot] = useState(() => item(changeId));
  const it = item(changeId) ?? snapshot;
  const ch = useChanger(snapshot);
  const [done, setDone] = useState<{ before: number; after: number; nofa: boolean } | null>(null);

  useLeaveGuard(ch);

  if (!it) return <><button className="btn ghost" onClick={() => go("issues")}>{t.back}</button></>;
  const step = done ? 4 : ch.phase === "generated" ? 2 : 1;

  const confirm = async () => {
    if (saving) return;
    setSaving(true);
    try {
      const before = report?.score ?? 0;
      const r = await api.changeConfirm(it.id);
      if (r) setReport(r);
      setState((await api.bootstrap()).state);
      ch.finish();
      setDone({ before, after: r?.score ?? before, nofa: has(it, "nofa") });
      celebrate();
    } catch (e) { fail(e); } finally { setSaving(false); }
  };

  return (
    <>
      <button className="btn ghost" onClick={() => go("issues")} style={{ marginBottom: 14 }}>{t.back}</button>
      <h1 className="title">{t.chTitle(it.title)}</h1>
      <p className="lead">{t.chLead}</p>
      <div className="steps">
        {t.chSteps.map((l, i) => (
          <span key={l} style={{ display: "contents" }}>
            {i > 0 && <span className="step-line" />}
            <span className={`step ${i + 1 < step ? "done" : i + 1 === step ? "now" : ""}`}><span className="dot">{i + 1 < step ? "✓" : i + 1}</span>{l}</span>
          </span>
        ))}
      </div>
      <div className="grid2">
        <div className="card">
          {done ? (
            <>
              <h2>{t.doneTitle}</h2>
              <p className="cap">{t.doneCap}</p>
              <div className="okbox"><b>{t.scoreUp(done.before, done.after)}</b><p>{done.nofa ? t.doneNext2fa : t.doneNext}</p></div>
              <div style={{ display: "flex", gap: 8, marginTop: 12, flexWrap: "wrap" }}>
                <button className="btn primary" onClick={() => go("issues")}>{t.toList}</button>
                {done.nofa && <button className="btn ghost" onClick={() => { const u = it.twofa_doc ?? siteUrl(it); if (u) api.openUrl(u).catch(fail); }}>{t.action.twofa}</button>}
              </div>
            </>
          ) : ch.phase !== "generated" ? (
            <>
              <h2>{t.genTitle}</h2>
              <p className="cap">{t.genCap(settings.pw_length, settings.pw_symbols)}</p>
              <div className="tags" style={{ marginBottom: 12 }}>{it.issues.map((x) => <Tag key={x.kind} k={x.kind} />)}</div>
              {it.passkey_site && <div className="warn">{t.passkeyWarn}</div>}
              <button className="btn coral" disabled={ch.phase === "busy"} onClick={() => void ch.generate()}>{ch.phase === "busy" ? "…" : t.genBtn}</button>
            </>
          ) : (
            <>
              <h2>{t.siteTitle}</h2>
              <p className="cap">{t.siteCap}</p>
              <ol className="how">{t.siteHow.map((s) => <li key={s}>{s}</li>)}</ol>
              <SiteActions ch={ch} />
              <div className="actions">
                <button className="btn primary" disabled={saving} onClick={confirm}>{t.saved}</button>
                <button className="btn ghost" onClick={() => void ch.restore()}>{t.undo}</button>
              </div>
            </>
          )}
        </div>
        <div className="card">
          <h2>{t.helpTitle}</h2>
          <p className="cap">{t.helpCap}</p>
          {t.help.map(([a, b]) => (
            <div className="set-row" key={a}><span><b>{a}</b><br /><small>{b}</small></span></div>
          ))}
        </div>
      </div>
    </>
  );
}
