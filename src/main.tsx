import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource-variable/geist";
import "@fontsource-variable/geist-mono";
import "./app.css";
import { App } from "./App";
import { AppProvider, applyTheme } from "./store";
import { getApi } from "./lib/api";

// 読み込み中に白く光らないよう、前回のテーマを先に当てる
try {
  const saved = localStorage.getItem("vs-theme");
  applyTheme(saved === "light" || saved === "dark" ? saved : "system");
} catch {
  applyTheme("system");
}

async function start() {
  const api = await getApi();
  const boot = await api.bootstrap();
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <AppProvider api={api} boot={boot}>
        <App />
      </AppProvider>
    </StrictMode>,
  );
}
void start();
