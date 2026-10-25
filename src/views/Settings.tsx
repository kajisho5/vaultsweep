import { useState } from "react";
import { useApp } from "../store";
import { Seg, Switch, fmtDate } from "../components/ui";
import { PendingBar } from "./Overview";
import type { Settings } from "../lib/types";
import { issueText } from "../lib/i18n";

export function SettingsView() {
  const { t, settings, saveSettings, nextRun, boot, api, toast, fail, report, errText } = useApp();
  const [schedErr, setSchedErr] = useState<string | null>(null);
  const [keyRef, setKeyRef] = useState(settings.hibp_key_ref ?? "");
  const [keyOk, setKeyOk] = useState<boolean | null>(null);
  const [conn, setConn] = useState<{ version: string; vaults: number; account: string | null } | { error: string } | null>(null);
  const [connBusy, setConnBusy] = useState(false);
  const testConn = async () => {
    setConnBusy(true);
    try { setConn(await api.checkConnection()); } catch (e) { setConn({ error: errText(e) }); } finally { setConnBusy(false); }
  };
  const up = async (patch: Partial<Settings>) => {
    const err = await saveSettings({ ...settings, ...patch });
    if ("auto_scan" in patch || "schedule" in patch || "schedule_time" in patch) setSchedErr(err);
  };
  const lang = settings.lang ?? "auto";
  const mcpCmd = boot.mcp_path ?? "vaultsweep-mcp";
  const config = JSON.stringify({ mcpServers: { vaultsweep: { command: mcpCmd, args: ["--mcp"] } } }, null, 2);

  const exportCsv = async () => {
    if (!report) return;
    const rows = report.items.filter((i) => i.issues.length).map((i) => [
      i.title, i.domain ?? "", i.vault, i.issues.map((x) => `${t.kind[x.kind]}: ${issueText(x, t)}`).join(" / "), String(Math.round(i.risk * 100)), i.updated_at ? fmtDate(i.updated_at, t.ymd) : "",
    ]);
    if (api.demo) { toast(t.demoExport); return; }
    try { toast(t.exported(await api.exportCsv(t.csvHead, rows))); } catch (e) { fail(e); }
  };

  return (
    <>
      <h1 className="title">{t.sTitle}</h1>
      <p className="lead">{t.sLead}</p>
      <PendingBar />
      <div className="settings">
        <div className="card">
          <h2>{t.sConn}</h2><p className="cap">{t.sConnCap}</p>
          {conn && "error" in conn && (
            <>
              <div className="warn">{conn.error}</div>
              <ol className="how" style={{ marginTop: 0 }}>{t.firstSteps.slice(0, 2).map((s) => <li key={s}>{s}</li>)}<li>{t.sConnInstall}</li></ol>
            </>
          )}
          {conn && !("error" in conn) && (
            <>
              <div className="okbox"><b>{t.sConnOk}</b></div>
              <div className="set-row">{t.sConnVer}<small className="m">{conn.version}</small></div>
              <div className="set-row">{t.sConnVaults}<small>{conn.vaults}</small></div>
              {conn.account && <div className="set-row">{t.sConnAcct}<small className="m">{conn.account}</small></div>}
            </>
          )}
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 8 }}>
            <button type="button" className="btn primary small" disabled={connBusy} aria-busy={connBusy} onClick={testConn}>{connBusy ? t.checking : t.sConnTest}</button>
            <button type="button" className="btn small" onClick={() => api.openUrl("https://www.1password.dev/cli/get-started").catch(fail)}>{t.setupLink}</button>
          </div>
        </div>

        <div className="card">
          <h2>{t.sView}</h2><p className="cap">{t.sViewCap}</p>
          <div className="set-row">{t.sLang}
            <select className="select" value={lang} aria-label={t.sLang} onChange={(e) => up({ lang: e.target.value === "auto" ? null : (e.target.value as "ja" | "en") })}>
              <option value="auto">{t.langAuto}</option><option value="ja">日本語</option><option value="en">English</option>
            </select>
          </div>
          <div className="set-row">{t.sTheme}
            <Seg label={t.sTheme} value={settings.theme} onChange={(v) => up({ theme: v })} options={[["light", t.themeLight], ["dark", t.themeDark], ["system", t.themeSystem]]} />
          </div>
        </div>

        <div className="card">
          <h2>{t.sAuto}</h2><p className="cap">{t.sAutoCap}</p>
          <div className="set-row">{t.sAutoOn}<Switch label={t.sAutoOn} on={settings.auto_scan} onChange={(v) => up({ auto_scan: v })} /></div>
          <div className="set-row">{t.sWhen}
            <span style={{ display: "flex", gap: 8 }}>
              <select className="select" value={settings.schedule} aria-label={t.sWhen} onChange={(e) => up({ schedule: e.target.value as Settings["schedule"] })}>
                {(["weekly", "daily", "monthly"] as const).map((s) => <option key={s} value={s}>{t.sched[s]}</option>)}
              </select>
              <input className="inp" type="time" aria-label={t.sTime} defaultValue={settings.schedule_time} key={settings.schedule_time}
                onBlur={(e) => { const v = e.target.value; if (v && v !== settings.schedule_time) void up({ schedule_time: v }); }}
                onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }} />
            </span>
          </div>
          <div className="set-row">{t.sNotify}
            <select className="select" value={settings.notify} aria-label={t.sNotify} onChange={(e) => up({ notify: e.target.value as Settings["notify"] })}>
              {(["new", "always", "never"] as const).map((s) => <option key={s} value={s}>{t.notify[s]}</option>)}
            </select>
          </div>
          <div className="set-row"><span>{t.sOffline}<br /><small>{t.sOfflineCap}</small></span><Switch label={t.sOffline} on={settings.offline} onChange={(v) => up({ offline: v })} /></div>
          <p className="cap" style={{ margin: "10px 0 0" }}>{t.sNext(nextRun ? t.dateTime(new Date(nextRun)) : null)}</p>
          {schedErr && <div className="warn">{t.sSchedErr(schedErr)}</div>}
        </div>

        <div className="card">
          <h2>{t.sRule}</h2><p className="cap">{t.sRuleCap}</p>
          <div className="set-row">{t.sStale}
            <select className="select" value={settings.stale_days} aria-label={t.sStale} onChange={(e) => up({ stale_days: Number(e.target.value) })}>
              {[180, 365, 730].map((d) => <option key={d} value={d}>{t.days(d)}</option>)}
            </select>
          </div>
          <div className="set-row">{t.sWeak}
            <select className="select" value={settings.weak_level} aria-label={t.sWeak} onChange={(e) => up({ weak_level: e.target.value as Settings["weak_level"] })}>
              {(["weak", "fair"] as const).map((d) => <option key={d} value={d}>{t.weakOpt[d]}</option>)}
            </select>
          </div>
          <h2 style={{ marginTop: 18 }}>{t.sGen}</h2><p className="cap">{t.sGenCap}</p>
          <div className="set-row">{t.sLen}
            <select className="select" value={settings.pw_length} aria-label={t.sLen} onChange={(e) => up({ pw_length: Number(e.target.value) })}>
              {[16, 20, 24, 32, 40].map((d) => <option key={d} value={d}>{t.chars(d)}</option>)}
            </select>
          </div>
          <div className="set-row">{t.sSym}<Switch label={t.sSym} on={settings.pw_symbols} onChange={(v) => up({ pw_symbols: v })} /></div>
        </div>

        <div className="card">
          <h2>{t.sMcp}</h2><p className="cap">{t.sMcpCap}</p>
          <div className="codebox m">{config}</div>
          <button type="button" className="btn small" onClick={() => api.copyPlain(config).then(() => toast(t.configCopied)).catch(fail)}>{t.copyConfig}</button>
          <div className="set-row">{t.sMcpGive}<small>{t.sMcpGiveV}</small></div>
          <div className="set-row">{t.sMcpNot}<small>{t.sMcpNotV}</small></div>
          <div className="set-row">{t.sMcpAsk}<small>{t.sMcpAskV}</small></div>
        </div>

        <div className="card">
          <h2>{t.sInt}</h2><p className="cap">{t.sIntCap}</p>
          <div style={{ padding: "12px 0" }}>
            <b style={{ fontSize: 14 }}>{t.sHibp}</b>
            <p className="cap" style={{ margin: "4px 0 8px" }}>{t.sHibpCap}</p>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <input className="inp m" style={{ flex: 1, minWidth: 200 }} placeholder={t.sHibpPh} value={keyRef} aria-label={t.sHibp}
                onChange={(e) => { setKeyRef(e.target.value); setKeyOk(null); }}
                onBlur={() => {
                  const v = keyRef.trim();
                  if (!v) { if (settings.hibp_key_ref) void up({ hibp_key_ref: null }); return; }
                  if (!v.startsWith("op://")) { toast(t.hibpBadRef); return; }
                  if (v !== settings.hibp_key_ref) void up({ hibp_key_ref: v });
                }} />
              <button type="button" className="btn small" disabled={!keyRef.trim().startsWith("op://")} onClick={async () => {
                try { setKeyOk(await api.testKeyRef(keyRef.trim())); await up({ hibp_key_ref: keyRef.trim() }); } catch (e) { setKeyOk(null); fail(e); }
              }}>{t.sHibpTest}</button>
              <button type="button" className="btn small" onClick={() => api.openUrl("https://haveibeenpwned.com/API/Key").catch(fail)}>{t.sHibpGet}</button>
            </div>
            {keyOk != null && <p className="cap" style={{ margin: "6px 0 0", color: keyOk ? "var(--color-ok)" : "var(--color-bad)" }}>{keyOk ? t.sHibpOk : t.sHibpNg}</p>}
          </div>
        </div>

        <div className="card">
          <h2>{t.sData}</h2><p className="cap">{t.sDataCap}</p>
          {t.src.map(([name, what, url]) => (
            <div className="src" key={name}><button type="button" className="linkbtn" onClick={() => api.openUrl(url).catch(fail)}>{name}</button><span>{what}</span></div>
          ))}
          <p className="cap" style={{ marginTop: 10 }}>{t.attribution}</p>
          <h2 style={{ marginTop: 18 }}>{t.sOther}</h2>
          <div className="set-row">{t.sExport}<button type="button" className="btn small" disabled={!report} onClick={exportCsv}>CSV</button></div>
          <div className="set-row">{t.sDataDir}<small className="m" style={{ overflowWrap: "anywhere", textAlign: "right" }}>{api.demo ? t.demoDir : boot.data_dir}</small></div>
          <div className="set-row">{t.sVersion}<small>{boot.version}</small></div>
        </div>
      </div>
    </>
  );
}
