import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createFullBackup,
  downloadFullBackup,
  listFullBackups,
  type BackupFile,
} from "../../api/client";
import EmptyState from "../../components/EmptyState";
import PageHeader from "../../components/PageHeader";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}

function formatTimestamp(timestamp: string): string {
  if (timestamp.length !== 16) return timestamp;
  return `${timestamp.slice(0, 4)}-${timestamp.slice(4, 6)}-${timestamp.slice(
    6,
    8,
  )} ${timestamp.slice(9, 11)}:${timestamp.slice(11, 13)}:${timestamp.slice(
    13,
    15,
  )} UTC`;
}

export default function BackupsView() {
  const queryClient = useQueryClient();
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const backups = useQuery({
    queryKey: ["full-backups"],
    queryFn: listFullBackups,
  });

  const create = useMutation({
    mutationFn: createFullBackup,
    onSuccess: (backup) => {
      setNotice(`Backup created: ${formatTimestamp(backup.timestamp)}`);
      setError(null);
      queryClient.invalidateQueries({ queryKey: ["full-backups"] });
    },
    onError: (createError) => {
      setNotice(null);
      setError((createError as Error).message);
    },
  });

  const download = async (file: BackupFile) => {
    try {
      const blob = await downloadFullBackup(file.name);
      const url = URL.createObjectURL(blob);
      const anchor = window.document.createElement("a");
      anchor.href = url;
      anchor.download = file.name;
      anchor.style.display = "none";
      window.document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      window.setTimeout(() => URL.revokeObjectURL(url), 1_000);
    } catch (downloadError) {
      setError((downloadError as Error).message);
    }
  };

  return (
    <section>
      <PageHeader
        title="Backups"
        description="Create and download encrypted backups without using a terminal."
        actions={
          <button
            type="button"
            disabled={create.isPending}
            onClick={() => create.mutate()}
          >
            {create.isPending ? "Backing up…" : "Back up now"}
          </button>
        }
      />

      {notice && <p className="notice">{notice}</p>}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {backups.isError && (
        <p className="error" role="alert">
          {(backups.error as Error).message}
        </p>
      )}

      {backups.isPending && <p className="muted">Loading backups…</p>}

      {backups.data && backups.data.length === 0 && (
        <EmptyState
          title="No backups yet"
          description="Create the first encrypted backup now. The backend writes it to the mounted ./backups directory."
          action={
            <button
              type="button"
              disabled={create.isPending}
              onClick={() => create.mutate()}
            >
              Create first backup
            </button>
          }
        />
      )}

      {backups.data && backups.data.length > 0 && (
        <div className="card">
          <div className="table-scroll">
            <table className="table">
              <thead>
                <tr>
                  <th>Backup</th>
                  <th>Size</th>
                  <th>Download</th>
                </tr>
              </thead>
              <tbody>
                {backups.data.map((backup) => (
                  <tr key={backup.timestamp}>
                    <td>
                      <strong>{formatTimestamp(backup.timestamp)}</strong>
                      <div className="muted">
                        {backup.documents
                          ? "Encrypted database + document archive"
                          : "Encrypted database"}
                      </div>
                    </td>
                    <td>
                      {formatBytes(backup.database.size_bytes)}
                      {backup.documents
                        ? ` + ${formatBytes(backup.documents.size_bytes)}`
                        : ""}
                    </td>
                    <td className="row-actions">
                      <button
                        type="button"
                        onClick={() => void download(backup.database)}
                      >
                        Database
                      </button>
                      {backup.documents && (
                        <button
                          type="button"
                          onClick={() => void download(backup.documents!)}
                        >
                          Documents
                        </button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <div className="card stack">
        <h3>How backups work</h3>
        <p className="muted">
          GUI backups run inside the backend container and are encrypted with{" "}
          <code>BACKUP_PASSPHRASE</code>. The files are written to the
          host-mounted <code>./backups</code> directory. Keep the passphrase
          separate from that directory.
        </p>
        <p className="muted">
          Scheduled and off-site backups still use the host timer (
          <code>scripts/scheduled-backup.sh</code>). Restore remains a deliberate
          host/CLI operation (<code>make restore FILE=...</code>) for now.
        </p>
        <p>
          See <code>docs/backup.md</code> for key rotation, retention, off-site
          storage, and restore drills.
        </p>
      </div>
    </section>
  );
}
