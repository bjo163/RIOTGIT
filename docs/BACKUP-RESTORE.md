# R!OTGIT — Backup and Restore Runbook

## Data model

The single-node Compose profile stores both SQLite metadata and bare Git repositories in one persistent named volume mounted at `/var/lib/riotgit`. A backup must capture the whole volume together; copying only `riotgit.db` or only `repositories/` is not a consistent recovery point.

## Create a backup

Run from the repository root with the Compose stack running:

```bash
bash scripts/backup.sh
```

The script:
1. resolves the actual Docker named volume from the API container;
2. stops API and web services to prevent writes during the snapshot;
3. archives the complete data volume;
4. writes a SHA-256 checksum when `sha256sum` is available;
5. restarts the services on exit.

Backups default to `./backups` (mode 0700 where supported). Override the directory with `RIOTGIT_BACKUP_DIR=/secure/off-host/path`. Copy backups and checksum files to encrypted off-host storage. A local backup on the same VPS is not disaster recovery.

## Verify an archive

```bash
sha256sum -c backups/riotgit-data-YYYYMMDDTHHMMSSZ.tar.gz.sha256
tar -tzf backups/riotgit-data-YYYYMMDDTHHMMSSZ.tar.gz | head
```

Keep the archive and checksum paired. Treat archives as confidential source-code and identity data.

## Restore

Restore replaces the complete contents of the R!OTGIT data volume. First copy the selected archive to the host and verify its checksum. Then run:

```bash
bash scripts/restore.sh ./backups/riotgit-data-YYYYMMDDTHHMMSSZ.tar.gz
```

The script requires the exact confirmation word `RESTORE`, stops API/web, creates a pre-restore safety snapshot, replaces the volume, and restarts the services. It rejects obvious absolute/parent-traversal archive paths before extraction. Only restore archives produced by the controlled R!OTGIT backup script; do not extract untrusted tar archives into the data volume.

After restore, verify:

```bash
curl --fail http://127.0.0.1:8787/health/ready
curl --fail http://127.0.0.1:8787/api/v1/meta
```

Then sign in through the web UI and test clone/fetch/push on a non-production test repository. Inspect the pre-restore safety snapshot if recovery must be rolled back.

## Important limits

- The scripts operate on the Compose named volume discovered from the API container; they do not back up external PostgreSQL or a separately mounted object store.
- The API and web services are stopped; unrelated services are not.
- Host disk space must be sufficient for the archive and safety snapshot.
- An archive/checksum is not a proven recovery point until a restore drill succeeds on a separate test volume or disposable host.
- Schedule regular off-host copies and periodic restore drills appropriate to your recovery objectives.
