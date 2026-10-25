import { useState } from "react";
import { useApp } from "../store";
import { siteUrl } from "../lib/api";

const today = () => {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
};

export function CancelFlow() {
  const { t, flowId, item, go, api, toast, fail, setState, confirmArchive, celebrate } = useApp();
  const [snap] = useState(() => item(flowId));
  const it = item(flowId) ?? snap;
  const [step, setStep] = useState(0);
  const [date, setDate] = useState(today());
  const [receipt, setReceipt] = useState("");
  const [written, setWritten] = useState(false);
  if (!it) return <button className="btn ghost" onClick={() => go("cancel")}>{t.cfBack}</button>;
  const site = siteUrl(it);
  const del = it.deletion;
  const open = (u: string | null) => { if (u) api.openUrl(u).catch(fail); };
  const mail = () => {
    if (!del?.email) return;
    const subject = del.email_subject || t.mailSubject;
    const body = t.mailBody(it.title) + (del.email_body ? `\n\n---\n${del.email_body}` : "");
    open(`mailto:${del.email}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`);
  };

  const bodies = [
    <>
      <h2>{t.cfDummyTitle}</h2>
      <p className="cap">{t.cfDummyCap}</p>
      <div className="dummy">
        {t.dummy.map(([k, v]) => (
          <span key={k} style={{ display: "contents" }}>
            <span>{k}</span><code>{v}</code>
            <button className="btn ghost small" onClick={() => api.copyPlain(v).then(() => toast(t.copiedPlain(k))).catch(fail)}>{t.copy}</button>
          </span>
        ))}
      </div>
      <div className="warn">{t.cfDummyWarn}</div>
      <button className="btn ghost" disabled={!site} onClick={() => open(site)}>{t.openProfile}</button>
    </>,
    <>
      <h2>{t.cfCardTitle}</h2>
      <p className="cap">{t.cfCardCap}</p>
      <div className="warn">{t.cfCardWarn}</div>
      <button className="btn ghost" disabled={!site} onClick={() => open(site)}>{t.openProfile}</button>
    </>,
    <>
      <h2>{t.cfLeaveTitle}</h2>
      {del ? <p className="cap">{t.cfLeaveCap(t.difficulty[del.difficulty] ?? del.difficulty)}</p> : <p className="cap">{t.cfLeaveNone}</p>}
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        {del?.url && <button className="btn primary" onClick={() => open(del.url)}>{t.openLeave}</button>}
        {del?.email && <button className={`btn ${del.url ? "ghost" : "primary"}`} onClick={mail}>{t.makeMail}</button>}
        {!del?.url && !del?.email && <button className="btn primary" disabled={!site} onClick={() => open(site)}>{t.openProfile}</button>}
      </div>
      {del?.email && <p className="cap" style={{ marginTop: 12 }}>{t.cfMailNote}</p>}
      {del?.notes && <p className="cap" style={{ marginTop: 8 }}>{t.cfNotes(del.notes)}</p>}
    </>,
    <>
      <h2>{t.cfProofTitle}</h2>
      <p className="cap">{t.cfProofCap}</p>
      <div className="dummy" style={{ gridTemplateColumns: "auto minmax(0,260px)" }}>
        <span>{t.leaveDate}</span><input className="inp" type="date" value={date} onChange={(e) => setDate(e.target.value)} />
        <span>{t.receiptNo}{t.optional}</span><input className="inp" value={receipt} maxLength={80} onChange={(e) => setReceipt(e.target.value)} />
      </div>
      <button className="btn ghost" disabled={written} onClick={async () => {
        try { setState(await api.writeReceipt(it.id, t.leaveDate, date, t.receiptNo, receipt)); setWritten(true); toast(t.proofWritten); } catch (e) { fail(e); }
      }}>{written ? "✓ " + t.proofWritten : t.writeProof}</button>
    </>,
    <>
      <h2>{t.cfArchTitle}</h2>
      <p className="cap">{t.cfArchCap}</p>
      <div className="cmd mono">op item delete &lt;ID&gt; --archive</div>
      <button className="btn coral" onClick={async () => {
        if (await confirmArchive(it.id, it.title)) { celebrate(); void go("cancel"); }
      }}>{t.archiveBtn}</button>
    </>,
  ];

  return (
    <>
      <button className="btn ghost" onClick={() => go("cancel")} style={{ marginBottom: 14 }}>{t.cfBack}</button>
      <h1 className="title">{t.cfTitle(it.title)}</h1>
      <p className="lead">{t.cfLead}</p>
      <div className="flow">
        <div className="card" style={{ padding: 12 }}>
          <ul className="flow-steps">
            {t.cfSteps.map((s, i) => (
              <li key={s} className={i < step ? "done" : i === step ? "now" : ""}>
                <button onClick={() => setStep(i)} aria-current={i === step ? "step" : undefined}><span className="dot">{i < step ? "✓" : i + 1}</span>{s}</button>
              </li>
            ))}
          </ul>
        </div>
        <div className="card">
          {bodies[step]}
          {step < bodies.length - 1 && (
            <div className="actions">
              <button className="btn primary" onClick={async () => {
                if (step === 2) { try { setState(await api.cancelSet(it.id, "left")); } catch { /* リストにない項目でも先へ */ } }
                setStep(step + 1);
              }}>{t.next}</button>
              <button className="btn ghost" onClick={() => setStep(step + 1)}>{t.skipStep}</button>
            </div>
          )}
        </div>
      </div>
    </>
  );
}
