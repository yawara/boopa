import type { DhcpGuidance, DhcpResponse } from "../services/api";

function ModeGuide({ mode, guide }: { mode: string; guide: DhcpGuidance }) {
  return (
    <div className="mode-guide">
      <div className="mode-guide-header">
        <h3>{mode}</h3>
        <span>{guide.architecture}</span>
      </div>
      <dl className="detail-grid">
        <div>
          <dt>Boot filename</dt>
          <dd>{guide.bootFilename}</dd>
        </div>
        <div>
          <dt>Next server</dt>
          <dd>{guide.nextServer}</dd>
        </div>
      </dl>
      <ul className="notes">
        {guide.notes.map((note) => (
          <li key={note}>{note}</li>
        ))}
      </ul>
      <div className="options">
        {guide.options.map((option) => (
          <article key={`${mode}-${option.key}`} className="option-chip">
            <strong>{option.key}</strong>
            <code>{option.value}</code>
            <p>{option.description}</p>
          </article>
        ))}
      </div>
    </div>
  );
}

export function DhcpGuideCard({ data }: { data: DhcpResponse }) {
  return (
    <section className="card card-wide">
      <div className="card-header">
        <p className="eyebrow">DHCP Guide</p>
        <h2>Manual settings for {data.selected}</h2>
      </div>
      <div className="mode-grid">
        <ModeGuide mode="BIOS" guide={data.bios} />
        <ModeGuide mode="UEFI" guide={data.uefi} />
      </div>
    </section>
  );
}
