import { DistroSelector } from "../components/DistroSelector";
import { CacheStatusCard } from "../components/CacheStatusCard";
import { DhcpGuideCard } from "../components/DhcpGuideCard";
import {
  useGetCacheQuery,
  useGetDhcpQuery,
  useGetDistrosQuery,
  useRefreshCacheMutation,
  useSetSelectionMutation,
} from "../services/api";

export function DashboardPage() {
  const distrosQuery = useGetDistrosQuery();
  const selected = distrosQuery.data?.selected;
  const dhcpQuery = useGetDhcpQuery(selected);
  const cacheQuery = useGetCacheQuery();
  const [setSelection, setSelectionState] = useSetSelectionMutation();
  const [refreshCache, refreshState] = useRefreshCacheMutation();

  if (distrosQuery.isLoading || !selected || !distrosQuery.data) {
    return <main className="shell">Loading dashboard...</main>;
  }

  return (
    <main className="shell">
      <header className="hero">
        <p className="eyebrow">Trusted LAN Control Plane</p>
        <h1>boopa</h1>
        <p className="hero-copy">
          Switch the active distro, inspect the DHCP values the network needs, and verify that the
          next BIOS and UEFI boots have cached assets ready.
        </p>
      </header>

      <section className="dashboard-grid">
        <DistroSelector
          current={selected}
          distros={distrosQuery.data.distros}
          isSaving={setSelectionState.isLoading}
          onChange={(distro) => {
            void setSelection(distro);
          }}
        />
        {cacheQuery.data ? (
          <CacheStatusCard
            distro={cacheQuery.data.selected}
            entries={cacheQuery.data.entries}
            isRefreshing={refreshState.isLoading}
            onRefresh={() => {
              void refreshCache(selected);
            }}
          />
        ) : (
          <section className="card">Loading cache state...</section>
        )}
      </section>

      {dhcpQuery.data ? (
        <DhcpGuideCard data={dhcpQuery.data} />
      ) : (
        <section className="card card-wide">Loading DHCP guidance...</section>
      )}
    </main>
  );
}
