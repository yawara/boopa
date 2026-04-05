import type { DistroId, DistroSummary } from "../services/api";

interface DistroSelectorProps {
  current: DistroId;
  distros: DistroSummary[];
  isSaving: boolean;
  onChange: (distro: DistroId) => void;
}

export function DistroSelector({
  current,
  distros,
  isSaving,
  onChange,
}: DistroSelectorProps) {
  return (
    <section className="card">
      <div className="card-header">
        <p className="eyebrow">Active Distribution</p>
        <h2>Choose what the LAN boots next</h2>
      </div>
      <label className="field">
        <span>Distro</span>
        <select
          aria-label="Distro"
          value={current}
          disabled={isSaving}
          onChange={(event) => onChange(event.target.value as DistroId)}
        >
          {distros.map((distro) => (
            <option key={distro.id} value={distro.id}>
              {distro.label}
            </option>
          ))}
        </select>
      </label>
      <p className="subtle">
        The selected distro is persisted by the backend and controls both DHCP guidance and
        boot asset refreshes.
      </p>
    </section>
  );
}
