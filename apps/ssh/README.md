# R!OTGIT OpenSSH Gateway

The OpenSSH gateway is an optional Compose profile. It reuses the Rust release-builder stage from the API Dockerfile so API and SSH artifacts are compiled once per shared BuildKit cache. It exposes only Git upload-pack and receive-pack through an SSH forced command; interactive shell, forwarding, agent forwarding, X11, tunnels, and password authentication are disabled.

## Enable

Set an SSH host port in the root `.env` if the default is unsuitable:

```bash
RIOTGIT_SSH_PORT=2222
```

Start the API and SSH gateway:

```bash
docker compose -f deploy/compose/docker-compose.yml --env-file .env --profile ssh up --build -d api ssh
```

The SSH port binds to loopback by default. For remote clients, explicitly publish/forward the selected port through a firewall or trusted network edge. The SSH host key is persisted in a dedicated Docker volume; back it up and protect it to avoid host identity changes.

## Register a public key

Use the web/API or CLI:

```bash
riotgit ssh-key add ~/.ssh/id_ed25519.pub
```

SSH keys are resolved from the same SQLite metadata used by the API. Revoking a key prevents new SSH authentications after the daemon's next authorization lookup.

## Clone and push

```bash
git clone ssh://git@your-host.example:2222/your-user/your-repo.git
```

The SSH user name is always `git`; the registered public key identifies the R!OTGIT user. The forced command accepts only native Git upload-pack and receive-pack requests and resolves repository paths from owner/name metadata, not arbitrary filesystem paths.

## Security design

- OpenSSH uses database-backed `AuthorizedKeysCommand` and per-key SHA-256 fingerprints.
- `ExposeAuthInfo` binds the forced command to the actual public key used for authentication.
- All sessions run the forced `riotgit-ssh` command; no shell is provided.
- Only Git upload-pack and receive-pack are accepted.
- Repository permissions are checked for every transport invocation.
- receive-pack uses a controlled global `pre-receive` hook to enforce the same branch-protection intent as Smart HTTP.
- SSH host keys use a dedicated persistent volume.
- The gateway shares the API data volume and service UID to preserve SQLite and bare repository ownership.

The first release of this gateway should be treated as **experimental** until the SSH smoke test passes against the deployment being used. Do not expose the port publicly without reviewing the host-key fingerprint and firewall policy.
