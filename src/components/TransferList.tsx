import {
  IconCheck,
  IconDownload,
  IconUpload,
  IconX,
  IconAlert,
  IconFolderOpen,
  IconTrash,
} from "../icons";

export interface TransferRow {
  id: string;
  direction: "incoming" | "outgoing";
  fileName: string;
  bytes: number;
  total: number;
  speed: number; // bytes/sec, 0 when unknown
  state: "transferring" | "done" | "declined" | "error";
}

interface Props {
  transfers: TransferRow[];
  onOpenFolder: () => void;
  onClear: () => void;
}

function humanSize(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let n = bytes;
  let i = 0;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i++;
  }
  return `${i === 0 ? n : n.toFixed(1)} ${units[i]}`;
}

function StateBadge({ state, pct }: { state: TransferRow["state"]; pct: number }) {
  if (state === "transferring")
    return <span className="badge badge-active">{pct}%</span>;
  if (state === "done")
    return (
      <span className="badge badge-done">
        <IconCheck size={13} /> Done
      </span>
    );
  if (state === "declined")
    return (
      <span className="badge badge-muted">
        <IconX size={13} /> Declined
      </span>
    );
  return (
    <span className="badge badge-error">
      <IconAlert size={13} /> Failed
    </span>
  );
}

export function TransferList({ transfers, onOpenFolder, onClear }: Props) {
  if (transfers.length === 0) return null;
  const hasFinished = transfers.some((t) => t.state !== "transferring");

  return (
    <section className="transfers" aria-label="Transfers">
      <div className="transfers-head">
        <h2 className="section-title">Transfers</h2>
        <div className="transfers-actions">
          <button type="button" className="icon-btn" onClick={onOpenFolder} title="Open download folder">
            <IconFolderOpen size={15} /> Folder
          </button>
          {hasFinished && (
            <button type="button" className="icon-btn" onClick={onClear} title="Clear finished transfers">
              <IconTrash size={15} /> Clear
            </button>
          )}
        </div>
      </div>

      <ul className="transfer-list">
        {transfers.map((t) => {
          const pct = t.total > 0 ? Math.round((t.bytes / t.total) * 100) : 0;
          const failed = t.state === "error" || t.state === "declined";
          const active = t.state === "transferring";
          return (
            <li className="transfer-row" key={t.id + t.fileName}>
              <span
                className={`transfer-dir transfer-dir-${t.direction}`}
                aria-label={t.direction === "incoming" ? "Receiving" : "Sending"}
              >
                {t.direction === "incoming" ? (
                  <IconDownload size={16} />
                ) : (
                  <IconUpload size={16} />
                )}
              </span>

              <div className="transfer-body">
                <div className="transfer-top">
                  <span className="transfer-name" title={t.fileName}>
                    {t.fileName || "…"}
                  </span>
                  <StateBadge state={t.state} pct={pct} />
                </div>
                <span
                  className="transfer-bar"
                  role="progressbar"
                  aria-valuenow={pct}
                  aria-valuemin={0}
                  aria-valuemax={100}
                >
                  <span
                    className={`transfer-bar-fill${t.state === "done" ? " is-done" : ""}${
                      failed ? " is-failed" : ""
                    }`}
                    style={{ width: `${failed ? 100 : pct}%` }}
                  />
                </span>
                <div className="transfer-meta">
                  <span>
                    {humanSize(t.bytes)}
                    {t.total > 0 && ` / ${humanSize(t.total)}`}
                  </span>
                  {active && t.speed > 0 && <span>{humanSize(t.speed)}/s</span>}
                </div>
              </div>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
