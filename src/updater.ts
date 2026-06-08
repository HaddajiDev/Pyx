import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type { Update };

export async function checkForUpdate(): Promise<Update | null> {
  try {
    return (await check()) ?? null;
  } catch {
    return null;
  }
}

export async function installUpdate(
  update: Update,
  onProgress?: (percent: number) => void,
): Promise<void> {
  let downloaded = 0;
  let total: number | null = null;
  await update.downloadAndInstall((event) => {
    if (event.event === "Started") {
      total = event.data.contentLength ?? null;
    } else if (event.event === "Progress") {
      downloaded += event.data.chunkLength;
      onProgress?.(total ? Math.round((downloaded / total) * 100) : -1);
    } else if (event.event === "Finished") {
      onProgress?.(100);
    }
  });
  await relaunch();
}
