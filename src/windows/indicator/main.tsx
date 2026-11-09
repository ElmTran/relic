import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../../styles/theme.css";
import { syncTheme } from "../../lib/ipc";
import { Indicator } from "./Indicator";

syncTheme();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Indicator />
  </StrictMode>,
);
