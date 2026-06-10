import {
  IconCheck,
  IconDownload,
  IconUpload,
  IconX,
  IconAlert,
  IconClock,
  IconFolderOpen,
  IconTrash,
} from "../icons";

export type TransferState =
  | "pending"
  | "transferring"
  | "done"
  | "declined"
  | "error";

export interface TransferFile {
  name: string;
  bytes: number;
  total: number;
}

export interface Transfer {
  id: string;
  direction: "incoming" | "outgoing";
  peer: string;
  state: TransferState;
  files: TransferFile[];
}

interface Props {
  transfers: Transfer[];
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

function Badge({ state, pct }: { state: TransferState; pct: number }) {
  if (state === "pending")
    return (
      <span className="badge badge-pending">
        <IconClock size={13} /> Waiting
      </span>
    );
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

function TransferCard({ t }: { t: Transfer }) {
  const totalBytes = t.files.reduce((a, f) => a + f.bytes, 0);
  const totalSize = t.files.reduce((a, f) => a + f.total, 0);
  const pct =
    t.state === "done" ? 100 : totalSize > 0 ? Math.round((totalBytes / totalSize) * 100) : 0;
  const incoming = t.direction === "incoming";
  const failed = t.state === "error";
  const showFiles =
    t.files.length > 0 && t.state !== "pending" && t.state !== "declined";
  const showBar = t.state === "transferring" || t.state === "done" || failed;

  return (
    <div className="xfer-card">
      <div className="xfer-head">
        <span className={`xfer-dir xfer-dir-${t.direction}`}>
          {incoming ? <IconDownload size={16} /> : <IconUpload size={16} />}
        </span>
        <div className="xfer-peer">
          <span className="xfer-peer-name">
            {incoming ? "From" : "To"} {t.peer || "device"}
          </span>
          <span className="xfer-peer-sub">
            {t.files.length > 0
              ? `${t.files.length} file${t.files.length === 1 ? "" : "s"} · ${humanSize(totalSize)}`
              : incoming
                ? "Receiving…"
                : "Preparing…"}
          </span>
        </div>
        <Badge state={t.state} pct={pct} />
      </div>

      {showBar && (
        <div className="xfer-overall">
          <span
            className={`xfer-bar-fill${t.state === "done" ? " is-done" : ""}${
              failed ? " is-failed" : ""
            }`}
            style={{ width: `${failed ? 100 : pct}%` }}
          />
        </div>
      )}

      {t.state === "pending" ? (
        <p className="xfer-note">
          Waiting for {t.peer || "the other device"} to accept…
        </p>
      ) : t.state === "declined" ? (
        <p className="xfer-note">
          {t.peer || "The other device"} declined the transfer.
        </p>
      ) : showFiles ? (
        <ul className="xfer-files">
          {t.files.map((f) => {
            const fp =
              t.state === "done"
                ? 100
                : f.total > 0
                  ? Math.round((f.bytes / f.total) * 100)
                  : 0;
            const fdone = fp >= 100;
            return (
              <li className="xfer-file" key={f.name}>
                {fdone ? (
                  <IconCheck size={13} className="xfer-file-tick" />
                ) : (
                  <span className="xfer-file-dot" />
                )}
                <span className="xfer-file-name" title={f.name}>
                  {f.name}
                </span>
                <span className="xfer-file-meta">
                  {fdone ? humanSize(f.total) : `${fp}%`}
                </span>
              </li>
            );
          })}
        </ul>
      ) : null}
    </div>
  );
}

function Group({ title, items }: { title: string; items: Transfer[] }) {
  if (items.length === 0) return null;
  return (
    <div className="xfer-group">
      <h3 className="xfer-group-title">
        {title} <span className="xfer-group-count">{items.length}</span>
      </h3>
      <div className="xfer-list">
        {items.map((t) => (
          <TransferCard key={t.id} t={t} />
        ))}
      </div>
    </div>
  );
}

export function TransferList({ transfers, onOpenFolder, onClear }: Props) {
  if (transfers.length === 0) return null;
  const outgoing = transfers.filter((t) => t.direction === "outgoing");
  const incoming = transfers.filter((t) => t.direction === "incoming");

  return (
    <section className="transfers" aria-label="Transfers">
      <div className="transfers-head">
        <h2 className="section-title">Transfers</h2>
        <div className="transfers-actions">
          <button type="button" className="icon-btn" onClick={onOpenFolder} title="Open download folder">
            <IconFolderOpen size={15} /> Folder
          </button>
          <button type="button" className="icon-btn" onClick={onClear} title="Clear finished transfers">
            <IconTrash size={15} /> Clear
          </button>
        </div>
      </div>

      <Group title="Sending" items={outgoing} />
      <Group title="Receiving" items={incoming} />
    </section>
  );
}
