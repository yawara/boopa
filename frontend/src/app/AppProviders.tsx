import type { ReactNode } from "react";
import { CssBaseline } from "@mui/material";
import { ThemeProvider } from "@mui/material/styles";
import { Provider } from "react-redux";

import type { AppStore } from "./store";
import { store as defaultStore } from "./store";
import { boopaTheme } from "../theme/boopaTheme";

interface AppProvidersProps {
  children: ReactNode;
  store?: AppStore;
}

export function AppProviders({
  children,
  store = defaultStore,
}: AppProvidersProps) {
  return (
    <ThemeProvider theme={boopaTheme}>
      <CssBaseline />
      <Provider store={store}>{children}</Provider>
    </ThemeProvider>
  );
}
