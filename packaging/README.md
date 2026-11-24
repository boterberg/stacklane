# Packaging and releases

| What | Where |
| --- | --- |
| Build the `.deb` | `packaging/build-deb.sh` |
| Release on a `v0.0.0.0` tag | `.github/workflows/release.yml` |
| First install on a server | `packaging/install.sh` |
| Reverse proxy | `packaging/Caddyfile.example` |

## Signing

Each release publishes `SHA256SUMS`. When the `release` environment has a signing key, the workflow also publishes `SHA256SUMS.asc`, a detached GPG signature. The environment only runs for `v*` tags.

One-time setup, on your own machine:

```sh
gpg --quick-gen-key "Stacklane releases <releases@stacklane.io>" ed25519 cert never
FPR=$(gpg --list-keys --with-colons releases@stacklane.io | awk -F: '$1 == "fpr" { print $10; exit }')
gpg --quick-add-key "$FPR" ed25519 sign 2y
gpg --armor --export "$FPR" > packaging/stacklane-release.asc
gpg --armor --export-secret-subkeys "$FPR" | gh secret set GPG_PRIVATE_KEY --env release -R boterberg/stacklane
gh secret set GPG_PASSPHRASE --env release -R boterberg/stacklane
gh variable set SIGNING_FPR --body "$FPR" -R boterberg/stacklane
gpg --armor --export-secret-keys "$FPR" > stacklane-master-OFFLINE.asc
```

Keep `stacklane-master-OFFLINE.asc` offline and delete it from the machine afterwards. Commit `packaging/stacklane-release.asc`: the package installs it as `/usr/share/keyrings/stacklane-release.gpg`.

Verify a release by hand:

```sh
gpg --import packaging/stacklane-release.asc
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum --check --ignore-missing SHA256SUMS
```
