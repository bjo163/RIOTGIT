# R!OTGIT CLI

The riotgit binary is the terminal companion to the R!OTGIT web forge.

## Build

    cargo build --release -p riotgit

The binary is written to target/release/riotgit.

## Sign in

    riotgit login --server https://git.example.com --username builder
    riotgit whoami

The password is prompted without echo and is not accepted as a command-line argument. A session token is stored in the operating-system config directory under riotgit/config.json. On Unix, the file is created with mode 0600. Protect the user account and home directory; use riotgit logout to revoke and remove the local session.

## Repositories

    riotgit repo list
    riotgit repo create --name notes --visibility private
    riotgit repo clone builder notes
    riotgit repo clone builder notes ./notes

Clone uses normal Git over HTTPS and does not write the bearer token into the repository remote URL or Git config. The token is passed to the Git child process through environment-based Git configuration.

## SSH public keys

    riotgit ssh-key list
    riotgit ssh-key add ~/.ssh/id_ed25519.pub
    riotgit ssh-key remove <key-id>

The API currently supports SSH key registration and revocation. OpenSSH clone/push transport is a separate implementation and is not enabled by key registration alone.

## Exit behavior

- Exit code 0: the requested operation completed.
- Non-zero: configuration, API, Git, or input failure. Error messages avoid printing access tokens.
