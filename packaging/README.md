# Packaging

`build-deb.sh` turns a built server, panel and website into `stacklane_<version>_amd64.deb`. The release workflow runs it for every `v0.0.0.0` tag and uploads the result to the package repository on stacklane.io; see `deploy/apt/`.

| File | Ends up as |
| --- | --- |
| `systemd/stacklane.service` | The service, running `stacklane serve` as the `stacklane` user |
| `systemd/stacklane-demo-reset.*` | Hourly demo reset, only enabled on the demo |
| `etc/stacklane.env` | `/etc/stacklane/stacklane.env`, kept on upgrades |
| `stacklane.asc` | `/usr/share/keyrings/stacklane.gpg`, the key the repository is signed with |
| `debian/` | Maintainer scripts and the control template |

Building by hand, on Debian or in a Debian container:

```sh
cargo build --release
(cd web && npm ci && npm run build)
(cd docs && npm ci && npm run build)
gpg --dearmor < packaging/stacklane.asc > stacklane.gpg
sh packaging/build-deb.sh --version 0.0.1.0 --bin target/release/stacklane \
  --web web/dist --site docs/dist --keyring stacklane.gpg --out dist
```
