# R!OTGIT — Backup and Restore Runbook

The application data volume stores SQLite metadata and bare Git repositories. The optional SSH profile uses a separate host-key volume to preserve OpenSSH server identity. Both must be protected when SSH is enabled.

## Backup

Run from the repository root:

```bash
bash scripts/backup.sh
```

The script discovers the named volumes from the running Compose project, records whether the SSH gateway is running, stops API/web/SSH writers, snapshots the full data volume and optional SSH host-key volume, writes SHA-256 checksums, and restarts only services that were running before the snapshot.

Backups default to `./backups` (mode 0700 where supported). Override with `RIOTGIT_BACKUP_DIR=/secure/off-host/path`. Copy archive/checksum pairs to encrypted off-host storage.

## Restore

```bash
bash scripts/restore.sh ./backups/riotgit-data-YYYYMMDDTHHMMSSZ.tar.gz
```

The companion SSH archive is discovered by filename. Restore validates obvious absolute/parent-traversal paths, requires the exact confirmation word `RESTORE`, stops active writers, restores the data volume and optional SSH identity volume, and restarts only previously running services.

After restore, verify:

```bash
curl --fail http://127.0.0.1:8787/health/ready
curl --fail http://127.0.0.1:8787/api/v1/meta
```

Then sign in and test clone/fetch/push. If SSH is enabled, verify the server host-key fingerprint and test an SSH clone/push. A separate-host restore drill is required before recovery can be called production-verified.

## Limits

External PostgreSQL databases and separate object stores are not covered. Backups are confidential source-code and identity data; store them encrypted off-host. A checksum alone is not proof of recoverability; perform periodic restore drills.
