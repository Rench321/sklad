import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
// Import local fonts
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/inter/700.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";

import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { SearchWindow } from "./SearchWindow";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {getCurrentWebviewWindow().label === "search" ? <SearchWindow /> : <App />}
  </React.StrictMode>,
);
