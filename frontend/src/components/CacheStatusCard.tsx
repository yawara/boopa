import type { CacheEntry, DistroId } from "../services/api";

interface CacheStatusCardProps {
  distro: DistroId;
  entries: CacheEntry[];
  isRefreshing: boolean;
  onRefresh: () => void;
}

export function CacheStatusCard({
  distro,
  entries,
  isRefreshing,
  onRefresh,
}: CacheStatusCardProps) {
  return (
    <section className="card">
      <div className="card-header">
        <p className="eyebrow">Cache State</p>
        <h2>Asset readiness for {distro}</h2>
      </div>
      <button className="action-button" disabled={isRefreshing} onClick={onRefresh}>
        {isRefreshing ? "Refreshing..." : "Refresh Selected Assets"}
      </button>
      <ul className="asset-list">
        {entries.map((entry) => (
          <li key={`${entry.bootMode}-${entry.relativePath}`} className="asset-row">
            <div>
              <strong>{entry.logicalName}</strong>
              <p>{entry.relativePath}</p>
            </div>
            <span className={`status-pill status-${entry.status}`}>{entry.status}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
