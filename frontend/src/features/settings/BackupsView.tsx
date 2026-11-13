import PageHeader from "../../components/PageHeader";

export default function BackupsView() {
  return (
    <section>
      <PageHeader
        title="Backups"
        description="Encrypted backups are managed from the host, not from the browser."
      />
      <div className="card stack">
        <h3>Manual and scheduled backups</h3>
        <p className="muted">
          Backups include the database and encrypted document archive. Keep the
          backup passphrase separate from the backup directory.
        </p>
        <pre className="code-block">{`make backup
make backup-scheduled
make restore FILE=backups/networth-....sql.gz.gpg
make restore-drill`}</pre>
        <p>
          See <code>docs/backup.md</code> for key separation, off-site copies,
          retention, and restore drills.
        </p>
      </div>
    </section>
  );
}
