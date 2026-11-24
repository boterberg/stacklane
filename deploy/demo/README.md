# demo.stacklane.io

The demo runs the normal package with `DEMO_MODE=true`. Visitors log in with one click as a customer or as an admin, and a timer puts the sample data back every hour.

```sh
sudo sh -c 'echo DEMO_MODE=true >> /etc/stacklane/stacklane.env'
sudo systemctl restart stacklane
sudo systemctl enable --now stacklane-demo-reset.timer
systemctl list-timers stacklane-demo-reset.timer
```

`stacklane-server demo-reset` empties every table except the departments and fills them again. Without `DEMO_MODE=true` it refuses to run, so a production install can never be wiped by accident.

The accounts it creates:

| Role | E-mail |
| --- | --- |
| Customer | klant@demo.stacklane.io |
| Admin | beheer@demo.stacklane.io |

Both get a random password; the demo buttons on the login page are the only way in.

`Caddyfile` in this folder is the configuration for demo.stacklane.io and stacklane.io.
