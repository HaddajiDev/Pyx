import { open } from "@tauri-apps/plugin-dialog";
import type { Peer } from "../types";
import { IconMonitor, IconSend, IconFolder } from "../icons";
import { Avatar } from "./Avatar";

interface Props {
  peer: Peer;
  dragOver: boolean;
  onSend: (peerId: string, paths: string[]) => void;
}

export function PeerCard({ peer, dragOver, onSend }: Props) {
  async function pickFiles() {
    const selected = await open({
      multiple: true,
      title: `Send files to ${peer.display_name}`,
    });
    if (!selected) return;
    onSend(peer.peer_id, Array.isArray(selected) ? selected : [selected]);
  }

  async function pickFolder() {
    const selected = await open({
      directory: true,
      multiple: true,
      title: `Send a folder to ${peer.display_name}`,
    });
    if (!selected) return;
    onSend(peer.peer_id, Array.isArray(selected) ? selected : [selected]);
  }

  return (
    <div
      className={`peer-card${dragOver ? " drag-over" : ""}`}
      data-peer-id={peer.peer_id}
      title={`${peer.display_name} · ${peer.addr}`}
    >
      <Avatar seed={peer.peer_id} name={peer.display_name} size={60} showStatus />
      <span className="peer-meta">
        <span className="peer-name">{peer.display_name}</span>
        <span className="peer-sub">
          <IconMonitor size={13} /> {peer.addr.split(":")[0]}
        </span>
      </span>
      <div className="peer-actions">
        <button
          type="button"
          className="peer-action"
          onClick={pickFiles}
          aria-label={`Send files to ${peer.display_name}`}
        >
          <IconSend size={13} /> Files
        </button>
        <button
          type="button"
          className="peer-action"
          onClick={pickFolder}
          aria-label={`Send a folder to ${peer.display_name}`}
        >
          <IconFolder size={13} /> Folder
        </button>
      </div>
    </div>
  );
}
