import React from "react";
import ReactDOM from "react-dom/client";
import "@mantine/core/styles.css";

import { AppProviders } from "./app/AppProviders";
import { DashboardPage } from "./pages/DashboardPage";
import "./styles/app.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <AppProviders>
      <DashboardPage />
    </AppProviders>
  </React.StrictMode>,
);
