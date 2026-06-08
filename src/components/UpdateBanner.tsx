import { IconDownload, IconX } from "../icons";

interface Props {
  version: string;
  installing: boolean;
  progress: number;
  onInstall: () => void;
  onDismiss: () => void;
}

export function UpdateBanner({ version, installing, progress, onInstall, onDismiss }: Props) {
  return (
    <div className="update-banner" role="status">
      <span className="update-icon" aria-hidden="true">
        <IconDownload size={16} />
      </span>
      <span className="update-text">
        {installing
          ? progress >= 0
            ? `Updating Pyx… ${progress}%`
            : "Updating Pyx…"
          : `Pyx ${version} is available`}
      </span>
      {!installing && (
        <>
          <button type="button" className="update-install" onClick={onInstall}>
            Update &amp; restart
          </button>
          <button
            type="button"
            className="update-dismiss"
            onClick={onDismiss}
            aria-label="Dismiss update"
          >
            <IconX size={15} />
          </button>
        </>
      )}
      {installing && progress >= 0 && (
        <span className="update-bar" aria-hidden="true">
          <span className="update-bar-fill" style={{ width: `${progress}%` }} />
        </span>
      )}
    </div>
  );
}
