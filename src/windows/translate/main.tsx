import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../../styles/theme.css";
import { syncTheme } from "../../lib/ipc";
import { TranslateWindow } from "./TranslateWindow";

syncTheme();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <TranslateWindow />
  </StrictMode>,
);
