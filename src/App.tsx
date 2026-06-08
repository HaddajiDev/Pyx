import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";
import type { Identity, Peer, IncomingOffer } from "./types";
import * as api from "./api";
import { PeerCard } from "./components/PeerCard";
import { OfferModal } from "./components/OfferModal";
import { TransferList, type TransferRow } from "./components/TransferList";
import { UpdateBanner } from "./components/UpdateBanner";
import { checkForUpdate, installUpdate, type Update } from "./updater";
import { IconRadar, IconFolder, IconFolderOpen, IconAlert } from "./icons";

interface Toast {
  id: number;
  message: string;
}

function folderName(path: string): string {
  const parts = path.split(/[/\\]/).filter(Boolean);
  return parts[parts.length - 1] || path;
}

function App() {
  const [identity, setIdentity] = useState<Identity | null>(null);
  const [peers, setPeers] = useState<Peer[]>([]);
  const [offer, setOffer] = useState<IncomingOffer | null>(null);
  const [transfers, setTransfers] = useState<TransferRow[]>([]);
  const [dragPeerId, setDragPeerId] = useState<string | null>(null);
  const [downloadDir, setDownloadDir] = useState<string>("");
  const [update, setUpdate] = useState<Update | null>(null);
  const [updating, setUpdating] = useState(false);
  const [updateProgress, setUpdateProgress] = useState(-1);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const toastSeq = useRef(0);
  const speedSamples = useRef<Map<string, { bytes: number; t: number }>>(new Map());

  function pushToast(message: string) {
    const id = ++toastSeq.current;
    setToasts((prev) => [...prev, { id, message }]);
    setTimeout(() => setToasts((prev) => prev.filter((t) => t.id !== id)), 5000);
  }

  function upsertTransfer(row: TransferRow) {
    setTransfers((prev) => {
      const i = prev.findIndex((t) => t.id === row.id);
      if (i === -1) return [...prev, row];
      const copy = [...prev];
      copy[i] = row;
      return copy;
    });
  }

  function setTransferState(id: string, state: TransferRow["state"]) {
    setTransfers((prev) => prev.map((t) => (t.id === id ? { ...t, state } : t)));
  }

  async function handleSend(peerId: string, paths: string[]) {
    if (paths.length === 0) return;
    await api.sendToPeer(peerId, paths);
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
    unlisteners.push(api.onIncomingOffer((o) => setOffer(o)));
    unlisteners.push(
      api.onProgress((p) => {
        const now = performance.now();
        const prev = speedSamples.current.get(p.transfer_id);
        let speed = 0;
        if (prev && now > prev.t) {
          speed = ((p.bytes - prev.bytes) / (now - prev.t)) * 1000;
          if (speed < 0) speed = 0;
        }
        speedSamples.current.set(p.transfer_id, { bytes: p.bytes, t: now });
        upsertTransfer({
          id: p.transfer_id,
          direction: p.direction,
          fileName: p.file_name,
          bytes: p.bytes,
          total: p.total,
          speed,
          state: "transferring",
        });
      }),
    );
    unlisteners.push(api.onTransferDone((id) => setTransferState(id, "done")));
    unlisteners.push(
      api.onTransferDeclined((id) => setTransferState(id, "declined")),
    );
    unlisteners.push(
      api.onTransferError((msg) => pushToast(`Transfer failed: ${msg}`)),
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

  function cardAt(pos: { x: number; y: number }): HTMLElement | null {
    const el = document.elementFromPoint(pos.x, pos.y);
    return (el?.closest("[data-peer-id]") as HTMLElement | null) ?? null;
  }

  async function init() {
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
    setOffer(null);
  }

  function clearFinished() {
    setTransfers((prev) => prev.filter((t) => t.state === "transferring"));
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
            <p className="hint">Drag files or folders onto a device, or click it to pick files.</p>
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
