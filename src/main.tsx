import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import { Companion } from "./Companion";
import "./styles.css";

const isCompanion =
  new URLSearchParams(window.location.search).has("companion") ||
  window.location.hash === "#companion";

if (isCompanion) {
  document.documentElement.classList.add("companion-window");
  document.body.classList.add("companion-window");
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>{isCompanion ? <Companion /> : <App />}</React.StrictMode>,
);
