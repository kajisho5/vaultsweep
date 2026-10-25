import { useApp } from "../store";
import { fmtDate } from "../components/ui";
import type { CancelEntry } from "../lib/types";

export function CancelBoard() {
  const { t, state, setState, api, item, startFlow, confirmArchive, toast, fail, report, setListFilter, setQuery, go } = useApp();
  const entries = Object.entries(state.cancel);
  const cols: CancelEntry["state"][] = ["todo", "left", "archived"];
  const open = entries.filter(([, e]) => e.state !== "archived").length;
  const staleNot = report?.items.filter((i) => i.issues.some((x) => x.kind === "stale") && !state.cancel[i.id]).length ?? 0;
  const set = async (id: string, s: "left" | "remove") => {
    try { setState(await api.cancelSet(id, s)); if (s === "left") toast(t.movedLeft); } catch (e) { fail(e); }
  };
  return (
    <>
      <h1 className="title">{t.cTitle(open)}</h1>
      <p className="lead">{t.cLead}</p>
      {staleNot > 0 && (
        <div className="bulkbar"><span>{t.cSuggest(staleNot)}</span>
          <button className="btn ghost" onClick={() => { setListFilter("stale"); setQuery(""); void go("issues"); }}>{t.cSuggestBtn}</button></div>
      )}
      <div className="kanban">
        {cols.map((c) => {
          const list = entries.filter(([, e]) => e.state === c).sort((a, b) => b[1].added_at.localeCompare(a[1].added_at));
          return (
            <div className="col" key={c}>
              <h3>{t.cCols[c]} <span>{list.length}</span></h3>
              {list.length === 0 && <p className="cap" style={{ margin: "0 4px" }}>{t.cEmpty}</p>}
              {list.map(([id, e]) => {
                const it = item(id);
                const diff = it?.deletion ? t.difficulty[it.deletion.difficulty] ?? it.deletion.difficulty : t.difficulty.none;
                const sub = c === "todo" ? diff : c === "left" ? t.cLeftAt(fmtDate(e.left_at, t.date)) : t.cArchivedAt(fmtDate(e.archived_at, t.date));
                return (
                  <div className="kcard" key={id} style={c === "archived" ? { opacity: 0.85 } : undefined}>
                    <h4>{e.title}</h4>
                    <small>{e.domain ?? "—"}{t.dot}{sub}</small>
                    <div className="btns">
                      {c === "todo" && <>
                        <button className="btn primary small" disabled={!it} onClick={() => startFlow(id)}>{t.cStart}</button>
                        <button className="btn ghost small" onClick={() => set(id, "left")}>{t.cLeft}</button>
                        <button className="linkbtn" style={{ fontSize: 12, color: "var(--ink3)" }} onClick={() => set(id, "remove")}>{t.cRemove}</button>
                      </>}
                      {c === "left" && <button className="btn coral small" disabled={!it} onClick={() => void confirmArchive(id, e.title)}>{t.cArchive}</button>}
                      {c === "archived" && <small>{t.cRestore}</small>}
                    </div>
                  </div>
                );
              })}
            </div>
          );
        })}
      </div>
    </>
  );
}
