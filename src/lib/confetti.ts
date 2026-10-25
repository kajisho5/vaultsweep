// 紙ふぶき（動きを減らす設定のときは出さない）
export function confetti() {
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  const c = document.getElementById("confetti") as HTMLCanvasElement | null;
  if (!c) return;
  const x = c.getContext("2d");
  if (!x) return;
  const dpr = Math.min(2, devicePixelRatio || 1);
  c.width = innerWidth * dpr; c.height = innerHeight * dpr; x.setTransform(dpr, 0, 0, dpr, 0, 0);
  const cols = ["#4F46E5", "#10B981", "#F59E0B", "#E5484D", "#3B82F6"];
  const ps = Array.from({ length: 140 }, () => ({
    x: innerWidth / 2, y: innerHeight * 0.35, vx: (Math.random() - 0.5) * 14, vy: -Math.random() * 12 - 4,
    r: Math.random() * 6 + 4, c: cols[Math.floor(Math.random() * cols.length)], a: Math.random() * 6, va: (Math.random() - 0.5) * 0.4,
  }));
  const t0 = performance.now();
  const step = (now: number) => {
    const t = (now - t0) / 1000;
    x.clearRect(0, 0, innerWidth, innerHeight);
    for (const p of ps) {
      p.vy += 0.35; p.vx *= 0.99; p.x += p.vx; p.y += p.vy; p.a += p.va;
      x.save(); x.translate(p.x, p.y); x.rotate(p.a); x.globalAlpha = Math.max(0, 1 - t / 2.4); x.fillStyle = p.c;
      x.fillRect(-p.r / 2, -p.r / 4, p.r, p.r / 2); x.restore();
    }
    if (t < 2.4) requestAnimationFrame(step); else x.clearRect(0, 0, innerWidth, innerHeight);
  };
  requestAnimationFrame(step);
}
