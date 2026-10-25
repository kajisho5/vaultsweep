// 紹介ページの小さな動き。どれも JS が無くても内容は読める。

// 画面に入ったら浮かび上がる（スクロールは IntersectionObserver で見る）
const io = new IntersectionObserver((ents) => {
  for (const e of ents) if (e.isIntersecting) { e.target.classList.add("in"); io.unobserve(e.target); }
}, { rootMargin: "0px 0px -10% 0px" });
document.querySelectorAll<HTMLElement>("[data-rv]").forEach((el) => io.observe(el));

// 大きな一言：言葉が読み線（画面の6割の高さ）を越えたら1つずつ色が付く
const wio = new IntersectionObserver((ents) => {
  for (const e of ents) if (e.isIntersecting) { e.target.classList.add("on"); wio.unobserve(e.target); }
}, { rootMargin: "0px 0px -40% 0px" });
document.querySelectorAll<HTMLElement>(".w").forEach((el, i) => { el.style.transitionDelay = `${(i % 4) * 60}ms`; wio.observe(el); });

// Mac で見ていたら、主ボタンを Mac 版にする（分からなければ Windows のまま）
const ua = navigator.userAgent;
const isMac = /Macintosh|Mac OS X/.test(ua) && !/iPhone|iPad/.test(ua);
if (isMac) {
  document.querySelectorAll<HTMLElement>("[data-dl]").forEach((box) => {
    box.querySelectorAll("[data-os=win]").forEach((el) => el.setAttribute("hidden", ""));
    box.querySelectorAll("[data-os=mac]").forEach((el) => el.removeAttribute("hidden"));
  });
}

// メニュー（スマホ）
const nav = document.querySelector<HTMLElement>("[data-nav]");
const burger = document.querySelector<HTMLButtonElement>("[data-burger]");
burger?.addEventListener("click", () => {
  const open = !nav?.hasAttribute("data-open");
  nav?.toggleAttribute("data-open", open);
  burger.setAttribute("aria-expanded", String(open));
  burger.setAttribute("aria-label", open ? burger.dataset.close! : burger.dataset.menu!);
  document.body.style.overflow = open ? "hidden" : "";
});
nav?.querySelectorAll(".sheet a").forEach((a) => a.addEventListener("click", () => burger?.click()));
addEventListener("keydown", (e) => { if (e.key === "Escape" && nav?.hasAttribute("data-open")) burger?.click(); });

// コマンドのコピー
document.querySelectorAll<HTMLButtonElement>("[data-copy]").forEach((b) => {
  b.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(b.dataset.copy!);
      const label = b.querySelector("[data-label]")!;
      const old = label.textContent;
      label.textContent = b.dataset.done!;
      setTimeout(() => { label.textContent = old; }, 1600);
    } catch { /* コピーできない環境では何もしない（コマンドは画面で選べる） */ }
  });
});
