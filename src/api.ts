import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Identity, Peer, IncomingOffer, ProgressEvent } from "./types";

export const getIdentity = () => invoke<Identity>("get_identity");
export const listPeers = () => invoke<Peer[]>("list_peers");
export const getDownloadDir = () => invoke<string>("get_download_dir");
export const setDownloadDir = (path: string) =>
  invoke<string>("set_download_dir", { path });
export const openDownloadDir = () => invoke("open_download_dir");
export const respondOffer = (transfer_id: string, accept: boolean) =>
  invoke("respond_offer", { transferId: transfer_id, accept });
export const sendToPeer = (peer_id: string, paths: string[]) =>
  invoke<string>("send_to_peer", { peerId: peer_id, paths });

export const onPeerFound = (cb: (p: Peer) => void): Promise<UnlistenFn> =>
  listen<Peer>("peer-found", (e) => cb(e.payload));
export const onPeerLost = (cb: (peerId: string) => void): Promise<UnlistenFn> =>
  listen<string>("peer-lost", (e) => cb(e.payload));
export const onIncomingOffer = (cb: (o: IncomingOffer) => void): Promise<UnlistenFn> =>
  listen<IncomingOffer>("incoming-offer", (e) => cb(e.payload));
export const onProgress = (cb: (p: ProgressEvent) => void): Promise<UnlistenFn> =>
  listen<ProgressEvent>("transfer-progress", (e) => cb(e.payload));
export const onTransferDone = (cb: (id: string) => void): Promise<UnlistenFn> =>
  listen<string>("transfer-done", (e) => cb(e.payload));
export const onTransferDeclined = (cb: (id: string) => void): Promise<UnlistenFn> =>
  listen<string>("transfer-declined", (e) => cb(e.payload));
export interface TransferErrorPayload {
  id?: string;
  error: string;
}
export const onTransferError = (
  cb: (p: TransferErrorPayload) => void,
): Promise<UnlistenFn> =>
  listen<TransferErrorPayload>("transfer-error", (e) => cb(e.payload));
