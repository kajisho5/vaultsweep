// 新しい版があるときに、概要の画面の一番上に出す帯
import { useApp } from "../store";
import { Icon } from "./ui";
import { installUpdate, laterUpdate, useUpdate } from "../lib/updater";

const NOTES = "https://github.com/kajisho5/vaultsweep/releases/latest";

export function UpdateBar() {
  const { t, api, fail } = useApp();
  const u = useUpdate();
  const show = !!u.info && u.later !== u.info.version && ["found", "downloading", "restarting", "blocked", "installFailed", "demo"].includes(u.status);
  if (!show || !u.info) return null;
  const busy = u.status === "downloading" || u.status === "restarting";
  const note = u.status === "blocked" ? t.upBlocked : u.status === "installFailed" ? t.upFail : u.status === "demo" ? t.upDemo : null;
  return (
    <div className="upbar" role="status">
      <span className="upicon"><Icon name="update" size={20} /></span>
      <div className="uptext">
        <b>{t.upTitle(u.info.version)}</b>
        <span>{t.upCap(u.info.current)}</span>
        {note && <span className="upnote">{note}</span>}
      </div>
      <div className="upbtns">
        <button type="button" className="btn" disabled={busy} onClick={laterUpdate}>{t.upLater}</button>
        <button type="button" className="btn" onClick={() => api.openUrl(NOTES).catch(fail)}>{t.upNotes}</button>
        <button type="button" className="btn" disabled={busy} aria-busy={busy} onClick={() => void installUpdate(api)}>
          <Icon name="download" size={16} />
          {u.status === "downloading" ? t.upDownloading(u.pct) : u.status === "restarting" ? t.upRestarting : t.upInstall}
        </button>
      </div>
    </div>
  );
}
