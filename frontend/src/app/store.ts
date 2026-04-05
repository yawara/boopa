import { configureStore } from "@reduxjs/toolkit";

import { networkBootApi } from "../services/api";

export const store = configureStore({
  reducer: {
    [networkBootApi.reducerPath]: networkBootApi.reducer,
  },
  middleware: (getDefaultMiddleware) =>
    getDefaultMiddleware().concat(networkBootApi.middleware),
});

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;

export function createTestStore() {
  return configureStore({
    reducer: {
      [networkBootApi.reducerPath]: networkBootApi.reducer,
    },
    middleware: (getDefaultMiddleware) =>
      getDefaultMiddleware().concat(networkBootApi.middleware),
  });
}
