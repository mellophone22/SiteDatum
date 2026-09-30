import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./global.css";
import { WorkspaceProvider } from "./workspace";
import { initializeTheme, ThemeProvider } from "./Theme";

initializeTheme();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider><WorkspaceProvider><App /></WorkspaceProvider></ThemeProvider>
  </React.StrictMode>,
);
