import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../../styles/theme.css";
import { syncTheme } from "../../lib/ipc";
import { Settings } from "./Settings";

syncTheme();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Settings />
  </StrictMode>,
);
