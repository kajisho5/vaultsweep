// 紹介ページ。GitHub Pages の https://kajisho5.github.io/vaultsweep/ に出す（英語が主、日本語は /ja/）
import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  site: "https://kajisho5.github.io",
  base: "/vaultsweep",
  trailingSlash: "always",
  vite: { plugins: [tailwindcss()] },
});
