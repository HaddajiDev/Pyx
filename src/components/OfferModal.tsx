import { useEffect, useRef } from "react";
import type { IncomingOffer } from "../types";
import { IconCheck, IconFile, IconX } from "../icons";

interface Props {
  offer: IncomingOffer;
  onRespond: (transferId: string, accept: boolean) => void;
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

export function OfferModal({ offer, onRespond }: Props) {
  const acceptRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    acceptRef.current?.focus();
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onRespond(offer.transfer_id, false);
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [offer.transfer_id, onRespond]);

  return (
    <div
      className="modal-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby="offer-title"
      onClick={() => onRespond(offer.transfer_id, false)}
    >
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <p className="modal-eyebrow">Incoming transfer</p>
        <h2 id="offer-title">
          <strong>{offer.from_name}</strong> wants to send you{" "}
          {offer.files.length} file{offer.files.length === 1 ? "" : "s"}
        </h2>
        <p className="modal-total">{humanSize(offer.total_size)} total</p>

        <ul className="offer-files">
          {offer.files.map((f) => (
            <li key={f.rel_path}>
              <IconFile size={16} className="offer-file-icon" />
              <span className="offer-file-name" title={f.rel_path}>
                {f.rel_path}
              </span>
              <span className="offer-file-size">{humanSize(f.size)}</span>
            </li>
          ))}
        </ul>

        <div className="modal-actions">
          <button
            type="button"
            className="btn btn-ghost"
            onClick={() => onRespond(offer.transfer_id, false)}
          >
            <IconX size={17} /> Decline
          </button>
          <button
            type="button"
            ref={acceptRef}
            className="btn btn-accept"
            onClick={() => onRespond(offer.transfer_id, true)}
          >
            <IconCheck size={17} /> Accept
          </button>
        </div>
      </div>
    </div>
  );
}
