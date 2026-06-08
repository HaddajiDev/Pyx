export interface Identity {
  peer_id: string;
  display_name: string;
}

export interface Peer {
  peer_id: string;
  display_name: string;
  addr: string;
}

export interface OfferedFile {
  name: string;
  rel_path: string;
  size: number;
}

export interface IncomingOffer {
  transfer_id: string;
  from_name: string;
  from_peer_id: string;
  files: OfferedFile[];
  total_size: number;
}

export interface ProgressEvent {
  transfer_id: string;
  direction: "incoming" | "outgoing";
  file_name: string;
  bytes: number;
  total: number;
}
