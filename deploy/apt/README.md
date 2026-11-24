# stacklane.io/apt

The package repository lives on the stacklane.io server and is managed with reprepro. GitHub only builds the package; signing happens on the server, with a key that never leaves it.

```
tag v0.0.1.0 ──> GitHub Actions builds the .deb
             ──> POST https://stacklane.io/apt/upload   (Bearer token)
             ──> receiver.py checks name and version, runs reprepro includedeb
             ──> reprepro signs Release and InRelease
```

## On the server

| Path | What |
| --- | --- |
| `/srv/stacklane-apt/conf/` | `distributions` and `options` from this folder |
| `/srv/stacklane-apt/public/` | What Caddy serves on `/apt/` |
| `/srv/stacklane-apt/.gnupg/` | The signing key, readable by `stacklane-repo` only |
| `/etc/stacklane-repo/token` | The upload token |
| `/usr/local/lib/stacklane-repo/receiver.py` | The upload receiver on `127.0.0.1:8091` |

The key fingerprint is `3DE6 4DD4 93CE C7A9 01BC 5B4B 5AAC 95B7 CE6B 5075`; its public half is `packaging/stacklane.asc`.

Caddy, inside the `stacklane.io` block:

```txt
handle /apt/upload {
	request_body {
		max_size 200MB
	}
	reverse_proxy 127.0.0.1:8091
}
handle_path /apt/* {
	root * /srv/stacklane-apt/public
	file_server
}
```

## Freshness

`distributions` sets `ValidFor: 30d`, so the signed index carries a `Valid-Until` date and apt refuses an old index that someone replays. The index therefore has to be signed again before it runs out, also when no release comes out. `stacklane-repo-export.timer` does that every day. Install the two units together with the new `distributions`, never the file alone, or every panel stops getting updates after 30 days:

```sh
sudo cp stacklane-repo-export.service stacklane-repo-export.timer /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now stacklane-repo-export.timer
sudo systemctl start stacklane-repo-export.service
```

## GitHub

The `release` environment holds one secret, `APT_UPLOAD_TOKEN`, and only runs for `v*` tags. Only the small `publish` job uses it; the build runs in a job without the environment, so dependency install scripts never share a machine with the token. The build also refuses a tag whose commit is not on `v1-stable`. To set it from the token on the server:

```sh
ssh ubuntu@stacklane.io sudo cat /etc/stacklane-repo/token | gh secret set APT_UPLOAD_TOKEN --env release -R boterberg/stacklane
```

## Removing a version

```sh
sudo -u stacklane-repo env HOME=/srv/stacklane-apt reprepro -b /srv/stacklane-apt remove stable stacklane
```
