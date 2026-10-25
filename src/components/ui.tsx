// 小さな部品
import { useEffect, useRef, useState, type CSSProperties } from "react";
import type { Kind } from "../lib/types";
import { useApp } from "../store";

/** 危険な種類（赤で見せるのはこの2つだけ） */
export const isBad = (k: Kind) => k === "leak" || k === "breach";

export function Tag({ k }: { k: Kind }) {
  const { t } = useApp();
  return <span className={`tag${isBad(k) ? " bad" : ""}`}>{t.kind[k]}</span>;
}

/** 0〜1 の量を、伸び縮み（transform）で見せる棒 */
export function Meter({ value, tone, thin, label }: { value: number; tone?: "cool"; thin?: boolean; label?: string }) {
  const v = Math.max(0, Math.min(1, value));
  return (
    <div className={`meter${tone ? " " + tone : ""}${thin ? " thin" : ""}`} role={label ? "img" : undefined} aria-label={label} aria-hidden={label ? undefined : true}>
      <i style={{ transform: `scaleX(${v})` } as CSSProperties} />
    </div>
  );
}

export function Logo({ size = 26 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 26 26" aria-hidden="true">
      <circle cx="13" cy="13" r="11.5" fill="none" stroke="currentColor" strokeWidth="1.6" />
      <circle cx="13" cy="13" r="6" fill="none" stroke="currentColor" strokeWidth="1" strokeDasharray="2 2" />
      <path d="M13 13 L22 7" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
      <circle cx="13" cy="13" r="2" fill="currentColor" />
    </svg>
  );
}

const NAV_ICON: Record<string, string> = {
  overview: '<rect x="1.5" y="1.5" width="5.5" height="5.5" rx="1.2"/><rect x="9" y="1.5" width="5.5" height="5.5" rx="1.2"/><rect x="1.5" y="9" width="5.5" height="5.5" rx="1.2"/><rect x="9" y="9" width="5.5" height="5.5" rx="1.2"/>',
  issues: '<path d="M8 1.8 L14.6 13.8 H1.4 Z"/><path d="M8 6.2 V9.4"/><circle cx="8" cy="11.6" r=".4"/>',
  cancel: '<rect x="2" y="2" width="12" height="12" rx="2"/><path d="M5.2 6 H10.8 M5.2 9.6 H9"/>',
  history: '<circle cx="8" cy="8" r="6.3"/><path d="M8 4.6 V8 L10.4 9.6"/>',
  settings: '<circle cx="8" cy="8" r="2.2"/><circle cx="8" cy="8" r="6" stroke-dasharray="2.6 1.8"/>',
};
export function NavIcon({ name }: { name: string }) {
  return <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" dangerouslySetInnerHTML={{ __html: NAV_ICON[name] }} />;
}

export function SearchIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true" style={{ flex: "none" }}>
      <circle cx="6" cy="6" r="4.5" fill="none" stroke="currentColor" strokeWidth="1.6" />
      <line x1="9.5" y1="9.5" x2="13" y2="13" stroke="currentColor" strokeWidth="1.6" />
    </svg>
  );
}

export function Lock() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
      <rect x="2" y="5" width="8" height="6" rx="1.5" fill="currentColor" />
      <path d="M4 5 V3.5 a2 2 0 0 1 4 0 V5" fill="none" stroke="currentColor" strokeWidth="1.4" />
    </svg>
  );
}

export function Switch({ on, onChange, label }: { on: boolean; onChange(v: boolean): void; label: string }) {
  return <button className="switch" role="switch" aria-checked={on} aria-label={label} onClick={() => onChange(!on)} />;
}

/** 数字を0から数え上げる */
export function CountUp({ value }: { value: number }) {
  const [v, setV] = useState(0);
  const from = useRef(0);
  useEffect(() => {
    const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const start = from.current, t0 = performance.now();
    let raf = 0;
    const step = (now: number) => {
      const k = reduce ? 1 : Math.min(1, (now - t0) / 900), e = 1 - Math.pow(1 - k, 3);
      const cur = Math.round(start + (value - start) * e);
      setV(cur);
      from.current = cur;
      if (k < 1) raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [value]);
  return <>{v}</>;
}

/** 線グラフ */
export function Spark({ values, height = 120, width = 400, label }: { values: number[]; height?: number; width?: number; label: string }) {
  const W = width, H = height, pad = 14;
  if (values.length < 2) return null;
  const lo = Math.max(0, Math.floor((Math.min(...values) - 5) / 10) * 10), hi = Math.min(100, Math.ceil((Math.max(...values) + 5) / 10) * 10);
  const span = Math.max(10, hi - lo);
  const x = (i: number) => pad + 14 + (i * (W - pad * 2 - 14)) / (values.length - 1);
  const y = (v: number) => H - pad - ((v - lo) / span) * (H - pad * 2);
  const pts = values.map((v, i) => `${x(i)},${y(v)}`).join(" ");
  const ticks: number[] = [];
  for (let v = lo; v <= lo + span; v += 10) ticks.push(v);
  const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
  return (
    <svg className="spark" viewBox={`0 0 ${W} ${H}`} role="img" aria-label={label}>
      {ticks.map((v) => (
        <g key={v}>
          <line className="gridline" x1="0" x2={W} y1={y(v)} y2={y(v)} />
          <text x="2" y={y(v) - 3}>{v}</text>
        </g>
      ))}
      <path className="area" d={`M${x(0)},${H - pad} L${pts.replace(/ /g, " L")} L${x(values.length - 1)},${H - pad} Z`} />
      <polyline key={values.join(",")} className="line" points={pts} pathLength={1} strokeDasharray="1 1" strokeDashoffset={reduce ? 0 : 1}>
        {!reduce && <animate attributeName="stroke-dashoffset" to="0" dur="1.4s" fill="freeze" begin="0.2s" />}
      </polyline>
      <circle className="dot" cx={x(values.length - 1)} cy={y(values[values.length - 1])} r="4" />
    </svg>
  );
}

/** コピーしてから消えるまでの残り時間 */
export function CopyTimer({ what, seconds, stamp }: { what: string | null; seconds: number; stamp: number }) {
  const { t } = useApp();
  const [left, setLeft] = useState(seconds);
  useEffect(() => {
    if (!what) return;
    setLeft(seconds);
    const id = window.setInterval(() => setLeft((l) => { if (l <= 1) { clearInterval(id); return 0; } return l - 1; }), 1000);
    return () => clearInterval(id);
  }, [what, seconds, stamp]);
  if (!what) return <div className="timer" />;
  return (
    <div className="timer" aria-live="polite">
      <span>{left > 0 ? t.copiedWhat(what) : t.cleared}</span>
      {left > 0 && <span className="left">{t.leftSec(left)}</span>}
      <Meter value={left / seconds} tone="cool" thin />
    </div>
  );
}



export function fmtDate(s: string | null | undefined, f: (d: Date) => string) {
  if (!s) return "—";
  const d = new Date(s);
  return isNaN(d.getTime()) ? "—" : f(d);
}
