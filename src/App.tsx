import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";
import type { Identity, Peer, IncomingOffer } from "./types";
import * as api from "./api";
import { PeerCard } from "./components/PeerCard";
import { OfferModal } from "./components/OfferModal";
import {
  TransferList,
  type Transfer,
  type TransferFile,
  type TransferState,
} from "./components/TransferList";
import { UpdateBanner } from "./components/UpdateBanner";
import { checkForUpdate, installUpdate, type Update } from "./updater";
import { ensureNotifyPermission, notify } from "./notify";
import { IconRadar, IconFolder, IconFolderOpen, IconAlert } from "./icons";

interface Toast {
  id: number;
  message: string;
}

function folderName(path: string): string {
  const parts = path.split(/[/\\]/).filter(Boolean);
  return parts[parts.length - 1] || path;
}

function plural(n: number): string {
  return n === 1 ? "" : "s";
}

function App() {
  const [identity, setIdentity] = useState<Identity | null>(null);
  const [peers, setPeers] = useState<Peer[]>([]);
  const [offer, setOffer] = useState<IncomingOffer | null>(null);
  const [transfers, setTransfers] = useState<Transfer[]>([]);
  const [dragPeerId, setDragPeerId] = useState<string | null>(null);
  const [downloadDir, setDownloadDir] = useState<string>("");
  const [update, setUpdate] = useState<Update | null>(null);
  const [updating, setUpdating] = useState(false);
  const [updateProgress, setUpdateProgress] = useState(-1);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const toastSeq = useRef(0);
  const peersRef = useRef<Peer[]>([]);
  peersRef.current = peers;
  const transfersRef = useRef<Transfer[]>([]);
  transfersRef.current = transfers;

  function pushToast(message: string) {
    const id = ++toastSeq.current;
    setToasts((prev) => [...prev, { id, message }]);
    setTimeout(() => setToasts((prev) => prev.filter((t) => t.id !== id)), 5000);
  }

  function upsertTransfer(t: Transfer) {
    setTransfers((prev) => {
      const i = prev.findIndex((x) => x.id === t.id);
      if (i === -1) return [...prev, t];
      const copy = [...prev];
      copy[i] = t;
      return copy;
    });
  }

  function patchTransfer(id: string, fn: (t: Transfer) => Transfer) {
    setTransfers((prev) => prev.map((t) => (t.id === id ? fn(t) : t)));
  }

  function setState(id: string, state: TransferState) {
    patchTransfer(id, (t) => ({ ...t, state }));
  }

  async function handleSend(peerId: string, paths: string[]) {
    if (paths.length === 0) return;
    const peer = peersRef.current.find((p) => p.peer_id === peerId);
    const peerName = peer?.display_name ?? "device";
    try {
      const id = await api.sendToPeer(peerId, paths);
      upsertTransfer({
        id,
        direction: "outgoing",
        peer: peerName,
        state: "pending",
        files: [],
      });
    } catch (e) {
      pushToast(`Couldn't start transfer: ${e}`);
    }
  }

  async function changeFolder() {
    const picked = await open({ directory: true, title: "Choose download folder" });
    if (!picked || Array.isArray(picked)) return;
    try {
      const set = await api.setDownloadDir(picked);
      setDownloadDir(set);
    } catch (e) {
      pushToast(`Couldn't set folder: ${e}`);
    }
  }

  useEffect(() => {
    init();
    const unlisteners: Promise<() => void>[] = [];

    unlisteners.push(
      api.onPeerFound((p) =>
        setPeers((prev) =>
          prev.some((x) => x.peer_id === p.peer_id) ? prev : [...prev, p],
        ),
      ),
    );
    unlisteners.push(
      api.onPeerLost((id) =>
        setPeers((prev) => prev.filter((p) => p.peer_id !== id)),
      ),
    );

    unlisteners.push(
      api.onIncomingOffer((o) => {
        setOffer(o);
        notify(
          `${o.from_name} wants to send you ${o.files.length} file${plural(o.files.length)}`,
          o.files.map((f) => f.name).slice(0, 4).join(", "),
        );
      }),
    );

    unlisteners.push(
      api.onProgress((p) =>
        patchOrCreateProgress(p.transfer_id, p.direction, p.file_name, p.bytes, p.total),
      ),
    );

    unlisteners.push(
      api.onTransferDone((id) => {
        const t = transfersRef.current.find((x) => x.id === id);
        if (t && t.direction === "incoming") {
          notify(
            "Files received",
            `${t.files.length} file${plural(t.files.length)} from ${t.peer}`,
          );
        }
        patchTransfer(id, (x) => ({
          ...x,
          state: "done",
          files: x.files.map((f) => ({ ...f, bytes: f.total })),
        }));
      }),
    );

    unlisteners.push(api.onTransferDeclined((id) => setState(id, "declined")));

    unlisteners.push(
      api.onTransferError((p) => {
        pushToast(`Transfer failed: ${p.error}`);
        if (p.id) setState(p.id, "error");
      }),
    );

    const dropUnlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        const card = cardAt(event.payload.position);
        setDragPeerId(card?.dataset.peerId ?? null);
      } else if (event.payload.type === "drop") {
        const card = cardAt(event.payload.position);
        const peerId = card?.dataset.peerId;
        if (peerId && event.payload.paths.length > 0) {
          handleSend(peerId, event.payload.paths);
        }
        setDragPeerId(null);
      } else {
        setDragPeerId(null);
      }
    });

    return () => {
      unlisteners.forEach((u) => u.then((fn) => fn()));
      dropUnlisten.then((fn) => fn());
    };
  }, []);

  function patchOrCreateProgress(
    id: string,
    direction: "incoming" | "outgoing",
    fileName: string,
    bytes: number,
    total: number,
  ) {
    setTransfers((prev) => {
      const i = prev.findIndex((t) => t.id === id);
      const updateFiles = (files: TransferFile[]): TransferFile[] => {
        const fi = files.findIndex((f) => f.name === fileName);
        if (fi === -1) return [...files, { name: fileName, bytes, total }];
        const copy = [...files];
        copy[fi] = { name: fileName, bytes, total };
        return copy;
      };
      if (i === -1) {
        return [
          ...prev,
          {
            id,
            direction,
            peer: "device",
            state: "transferring",
            files: [{ name: fileName, bytes, total }],
          },
        ];
      }
      const t = prev[i];
      const copy = [...prev];
      copy[i] = {
        ...t,
        state: t.state === "done" ? "done" : "transferring",
        files: updateFiles(t.files),
      };
      return copy;
    });
  }

  function cardAt(pos: { x: number; y: number }): HTMLElement | null {
    const el = document.elementFromPoint(pos.x, pos.y);
    return (el?.closest("[data-peer-id]") as HTMLElement | null) ?? null;
  }

  async function init() {
    ensureNotifyPermission();
    setIdentity(await api.getIdentity());
    setPeers(await api.listPeers());
    try {
      setDownloadDir(await api.getDownloadDir());
    } catch {
      setDownloadDir("");
    }
    checkForUpdate().then((u) => {
      if (u) setUpdate(u);
    });
  }

  async function runUpdate() {
    if (!update) return;
    setUpdating(true);
    setUpdateProgress(-1);
    try {
      await installUpdate(update, (p) => setUpdateProgress(p));
    } catch (e) {
      pushToast(`Update failed: ${e}`);
      setUpdating(false);
    }
  }

  function respond(transferId: string, accept: boolean) {
    api.respondOffer(transferId, accept);
    if (accept && offer) {
      upsertTransfer({
        id: offer.transfer_id,
        direction: "incoming",
        peer: offer.from_name,
        state: "transferring",
        files: offer.files.map((f) => ({ name: f.rel_path, bytes: 0, total: f.size })),
      });
    }
    setOffer(null);
  }

  function clearFinished() {
    setTransfers((prev) =>
      prev.filter((t) => t.state === "pending" || t.state === "transferring"),
    );
  }

  return (
    <div className="app">
      <div className="app-glow" aria-hidden="true" />

      <header className="app-header">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            <img src="/pyx-logo.png" alt="" />
          </span>
          <div className="brand-text">
            <span className="brand-name">Pyx</span>
            <span className="brand-tag">Peer-to-peer over your LAN</span>
          </div>
        </div>
        <div className="identity">
          <span className="status-pill">
            <span className="status-dot" />
            {identity ? "Listening" : "Starting…"}
          </span>
          {identity && <span className="identity-name">{identity.display_name}</span>}
        </div>
      </header>

      {update && (
        <UpdateBanner
          version={update.version}
          installing={updating}
          progress={updateProgress}
          onInstall={runUpdate}
          onDismiss={() => setUpdate(null)}
        />
      )}

      <main className="app-main">
        {peers.length === 0 ? (
          <div className="radar-empty">
            <div className="radar" aria-hidden="true">
              <span className="radar-ring radar-ring-1" />
              <span className="radar-ring radar-ring-2" />
              <span className="radar-ring radar-ring-3" />
              <span className="radar-grid" />
              <span className="radar-ping" />
              <span className="radar-ping radar-ping-2" />
              <span className="radar-ping radar-ping-3" />
              <span className="radar-sweep" />
              <span className="radar-blip radar-blip-1" />
              <span className="radar-blip radar-blip-2" />
              <span className="radar-blip radar-blip-3" />
              <span className="radar-core">
                <IconRadar size={26} />
              </span>
            </div>
            <h2 className="radar-title">Looking for peers…</h2>
            <p className="radar-sub">
              Open Pyx on another device on the same network and it’ll appear
              here.
            </p>
          </div>
        ) : (
          <section aria-label="Discovered peers">
            <h2 className="section-title">
              Nearby devices <span className="count-chip">{peers.length}</span>
            </h2>
            <div className="peer-grid">
              {peers.map((p) => (
                <PeerCard
                  key={p.peer_id}
                  peer={p}
                  dragOver={dragPeerId === p.peer_id}
                  onSend={handleSend}
                />
              ))}
            </div>
            <p className="hint">
              Drag files or folders onto a device, or use its Files / Folder buttons.
            </p>
          </section>
        )}

        <TransferList
          transfers={transfers}
          onOpenFolder={() => api.openDownloadDir()}
          onClear={clearFinished}
        />
      </main>

      <footer className="app-footer">
        <div className="save-to">
          <IconFolder size={15} />
          <span className="save-to-label">Saving to</span>
          <button
            type="button"
            className="save-to-path"
            onClick={() => api.openDownloadDir()}
            title={downloadDir}
          >
            {downloadDir ? folderName(downloadDir) : "…"}
          </button>
        </div>
        <button type="button" className="icon-btn" onClick={changeFolder}>
          <IconFolderOpen size={15} /> Change
        </button>
      </footer>

      <div className="toast-stack" aria-live="polite">
        {toasts.map((t) => (
          <div className="toast" key={t.id} role="status">
            <IconAlert size={16} />
            <span>{t.message}</span>
          </div>
        ))}
      </div>

      {offer && <OfferModal offer={offer} onRespond={respond} />}
    </div>
  );
}

export default App;
