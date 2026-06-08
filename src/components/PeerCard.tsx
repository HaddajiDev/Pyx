import { open } from "@tauri-apps/plugin-dialog";
import type { Peer } from "../types";
import { IconMonitor, IconSend } from "../icons";
import { Avatar } from "./Avatar";

interface Props {
  peer: Peer;
  dragOver: boolean;
  onSend: (peerId: string, paths: string[]) => void;
}

export function PeerCard({ peer, dragOver, onSend }: Props) {
  async function pickAndSend() {
    const selected = await open({ multiple: true, title: `Send to ${peer.display_name}` });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    onSend(peer.peer_id, paths);
  }

  return (
    <button
      type="button"
      className={`peer-card${dragOver ? " drag-over" : ""}`}
      data-peer-id={peer.peer_id}
      onClick={pickAndSend}
      aria-label={`Send files to ${peer.display_name} at ${peer.addr}`}
      title={`${peer.display_name} · ${peer.addr}`}
    >
      <Avatar seed={peer.peer_id} name={peer.display_name} size={60} showStatus />
      <span className="peer-meta">
        <span className="peer-name">{peer.display_name}</span>
        <span className="peer-sub">
          <IconMonitor size={13} /> {peer.addr.split(":")[0]}
        </span>
      </span>
      <span className="peer-send-hint" aria-hidden="true">
        <IconSend size={15} />
        <span>Send</span>
      </span>
    </button>
  );
}
