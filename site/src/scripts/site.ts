// 紹介ページの小さな動き。どれも JS が無くても内容は読める。
const root = document.documentElement;

// 画面に入ったら浮かび上がる（スクロールは IntersectionObserver で見る）
if ("IntersectionObserver" in window) {
  const io = new IntersectionObserver((ents) => {
    for (const e of ents) if (e.isIntersecting) { e.target.classList.add("in"); io.unobserve(e.target); }
  }, { rootMargin: "0px 0px -10% 0px" });
  document.querySelectorAll<HTMLElement>("[data-rv]").forEach((el) => io.observe(el));

  // 大きな一言：言葉が読み線（画面の6割の高さ）を越えたら1つずつ色が付く
  const wio = new IntersectionObserver((ents) => {
    for (const e of ents) if (e.isIntersecting) { e.target.classList.add("on"); wio.unobserve(e.target); }
  }, { rootMargin: "0px 0px -40% 0px" });
  document.querySelectorAll<HTMLElement>(".w").forEach((el, i) => { el.style.transitionDelay = `${(i % 4) * 60}ms`; wio.observe(el); });
}
// 準備できた印（3秒たっても付かなければ、演出をやめて全部見せる。Layout.astro）
root.dataset.ready = "1";

// Mac で見ていたら、主ボタンを Mac 版にする（分からなければ Windows のまま）。
// iPad は Mac と同じ名乗り方をするので、指で触れる画面かどうかで外す
const ua = navigator.userAgent;
const isMac = /Macintosh|Mac OS X/.test(ua) && !/iPhone|iPad/.test(ua) && navigator.maxTouchPoints <= 1;
if (isMac) {
  document.querySelectorAll<HTMLElement>("[data-dl]").forEach((box) => {
    box.querySelectorAll("[data-os=win]").forEach((el) => el.setAttribute("hidden", ""));
    box.querySelectorAll("[data-os=mac]").forEach((el) => el.removeAttribute("hidden"));
  });
}

// メニュー（スマホ）。開いている間は裏のページを触れなくし、閉じたら ≡ ボタンへ戻る
const nav = document.querySelector<HTMLElement>("[data-nav]");
const burger = document.querySelector<HTMLButtonElement>("[data-burger]");
const sheet = document.querySelector<HTMLElement>("#sheet");
const behind = Array.from(document.querySelectorAll<HTMLElement>("main, footer, body > a[href='#main'], [data-nav] nav > :not([data-burger])"));
function setMenu(open: boolean, returnFocus = true) {
  if (!nav || !burger || !sheet) return;
  nav.toggleAttribute("data-open", open);
  sheet.inert = !open;
  behind.forEach((el) => { el.inert = open; });
  burger.setAttribute("aria-expanded", String(open));
  burger.setAttribute("aria-label", open ? burger.dataset.close! : burger.dataset.menu!);
  root.style.overflow = open ? "hidden" : "";
  document.body.style.overflow = open ? "hidden" : "";
  if (open) sheet.querySelector<HTMLElement>("a")?.focus();
  else if (returnFocus) burger.focus();
}
burger?.addEventListener("click", () => setMenu(!nav?.hasAttribute("data-open")));
sheet?.querySelectorAll("a").forEach((a) => a.addEventListener("click", () => setMenu(false, false)));
addEventListener("keydown", (e) => {
  if (!nav?.hasAttribute("data-open")) return;
  if (e.key === "Escape") { setMenu(false); return; }
  // メニューの中（≡ ボタンとリンク）だけを Tab で回る
  if (e.key === "Tab" && sheet && burger) {
    const items = [burger, ...Array.from(sheet.querySelectorAll<HTMLElement>("a"))];
    const i = items.indexOf(document.activeElement as HTMLElement);
    const next = e.shiftKey ? (i <= 0 ? items.length - 1 : i - 1) : (i === -1 || i === items.length - 1 ? 0 : i + 1);
    e.preventDefault();
    items[next].focus();
  }
});
// 画面を広げてパソコン幅になったら、メニューは閉じる（スクロールできないままにしない）
matchMedia("(min-width: 48rem)").addEventListener("change", (m) => { if (m.matches && nav?.hasAttribute("data-open")) setMenu(false, false); });

// コマンドのコピー
document.querySelectorAll<HTMLButtonElement>("[data-copy]").forEach((b) => {
  const label = b.querySelector<HTMLElement>("[data-label]")!;
  const original = label.textContent ?? "";
  let timer = 0;
  b.addEventListener("click", async () => {
    let ok = true;
    try { await navigator.clipboard.writeText(b.dataset.copy!); } catch { ok = false; }
    label.textContent = ok ? b.dataset.done! : b.dataset.fail!;
    clearTimeout(timer);
    timer = window.setTimeout(() => { label.textContent = original; }, 1600);
  });
});
