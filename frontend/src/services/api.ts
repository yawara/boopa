import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";

export type DistroId = "ubuntu" | "fedora" | "arch";

export interface DistroSummary {
  id: DistroId;
  label: string;
}

export interface DistrosResponse {
  selected: DistroId;
  distros: DistroSummary[];
}

export interface DhcpOptionHint {
  key: string;
  value: string;
  description: string;
}

export interface DhcpGuidance {
  bootFilename: string;
  nextServer: string;
  architecture: string;
  notes: string[];
  options: DhcpOptionHint[];
}

export interface DhcpResponse {
  selected: DistroId;
  bios: DhcpGuidance;
  uefi: DhcpGuidance;
}

export interface CacheEntry {
  distroId: DistroId;
  bootMode: "bios" | "uefi";
  logicalName: string;
  sourceUrl: string;
  relativePath: string;
  localPath: string;
  status: "missing" | "cached" | "refreshed";
  lastSyncedAt: number | null;
}

export interface CacheResponse {
  selected: DistroId;
  entries: CacheEntry[];
}

export interface SelectionResponse {
  selected: DistroId;
}

const apiBaseUrl =
  typeof globalThis.location?.origin === "string"
    ? new URL("/api/", globalThis.location.origin).toString()
    : "http://localhost/api/";

export const networkBootApi = createApi({
  reducerPath: "networkBootApi",
  baseQuery: fetchBaseQuery({ baseUrl: apiBaseUrl }),
  tagTypes: ["Distros", "Dhcp", "Cache"],
  endpoints: (builder) => ({
    getDistros: builder.query<DistrosResponse, void>({
      query: () => "distros",
      providesTags: ["Distros"],
    }),
    getDhcp: builder.query<DhcpResponse, DistroId | undefined>({
      query: (distro) => (distro ? `dhcp?distro=${distro}` : "dhcp"),
      providesTags: ["Dhcp"],
    }),
    getCache: builder.query<CacheResponse, void>({
      query: () => "cache",
      providesTags: ["Cache"],
    }),
    setSelection: builder.mutation<SelectionResponse, DistroId>({
      query: (distro) => ({
        url: "selection",
        method: "PUT",
        body: { distro },
      }),
      invalidatesTags: ["Distros", "Dhcp", "Cache"],
    }),
    refreshCache: builder.mutation<CacheResponse, DistroId | undefined>({
      query: (distro) => ({
        url: "cache/refresh",
        method: "POST",
        body: distro ? { distro } : undefined,
      }),
      invalidatesTags: ["Cache"],
    }),
  }),
});

export const {
  useGetCacheQuery,
  useGetDhcpQuery,
  useGetDistrosQuery,
  useRefreshCacheMutation,
  useSetSelectionMutation,
} = networkBootApi;
