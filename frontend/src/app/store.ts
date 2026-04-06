import { configureStore } from "@reduxjs/toolkit";

import { networkBootApi } from "../services/api";

export function createAppStore() {
  return configureStore({
    reducer: {
      [networkBootApi.reducerPath]: networkBootApi.reducer,
    },
    middleware: (getDefaultMiddleware) =>
      getDefaultMiddleware().concat(networkBootApi.middleware),
  });
}

export const store = createAppStore();

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;
export type AppStore = ReturnType<typeof createAppStore>;

export function createTestStore() {
  return createAppStore();
}
