import type { ReactNode } from "react";
import { MantineProvider } from "@mantine/core";
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
    <MantineProvider defaultColorScheme="light" theme={boopaTheme}>
      <Provider store={store}>{children}</Provider>
    </MantineProvider>
  );
}
