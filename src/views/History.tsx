import { useApp } from "../store";
import { Spark, fmtDate } from "../components/ui";

export function History() {
  const { t, state } = useApp();
  const h = state.history;
  const d = h.length >= 2 ? h[h.length - 1].score - h[0].score : 0;
  return (
    <>
      <h1 className="title">{t.hTitle(h.length, d)}</h1>
      <p className="lead">{t.hLead}</p>
      <div className="card" style={{ marginBottom: 16 }}>
        <h2>{t.hChart}</h2>
        <p className="cap">{h.length >= 2 ? t.hChartCap : t.trendFew}</p>
        <Spark values={h.slice(-24).map((x) => x.score)} height={170} width={900} label={t.hChart} />
      </div>
      <table className="tbl">
        <thead><tr><th>{t.thDate}</th><th>{t.thScore}</th><th>{t.thFlagged}</th><th className="hide-sm">{t.thNew}</th></tr></thead>
        <tbody>
          {h.length === 0 && <tr><td colSpan={4} style={{ color: "var(--ink3)" }}>{t.firstTitle}</td></tr>}
          {[...h].reverse().map((x, i, arr) => {
            const prev = arr[i + 1];
            const dd = prev ? x.score - prev.score : null;
            return (
              <tr key={x.at + i}>
                <td>{fmtDate(x.at, t.date)}</td>
                <td><b>{x.score}</b> {dd != null && <small style={{ color: dd >= 0 ? "var(--ok)" : "var(--c-leak)" }}>{dd >= 0 ? "+" : ""}{dd}</small>}</td>
                <td>{t.countUnit(x.flagged)}</td>
                <td className="hide-sm">{x.new_issues ? t.countUnit(x.new_issues) : t.none}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </>
  );
}
