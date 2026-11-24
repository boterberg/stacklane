<script setup lang="ts">
import Icon from './Icon.vue'

const install = `curl -fsSL https://stacklane.io/apt/stacklane.gpg \\
  | sudo tee /usr/share/keyrings/stacklane.gpg > /dev/null
echo "deb [signed-by=/usr/share/keyrings/stacklane.gpg] \\
  https://stacklane.io/apt stable main" \\
  | sudo tee /etc/apt/sources.list.d/stacklane.list
sudo apt update
sudo apt install stacklane
sudo stacklane setup`

const reasons = [
  {
    icon: 'package',
    title: 'Eén programma',
    text: 'Server, API en klantpanel zitten samen in de binary /usr/bin/stacklane. Geen PHP, geen Node en geen queue-worker die je apart moet bewaken.',
  },
  {
    icon: 'shield-check',
    title: 'Als systemd-service',
    text: 'De service draait als eigen gebruiker zonder rechten op de rest van het systeem. Logboeken lees je met journalctl -u stacklane.',
  },
  {
    icon: 'database',
    title: 'PostgreSQL',
    text: 'Bij elke start voert Stacklane openstaande migraties uit. Je hoeft na een update niets met de hand te doen.',
  },
  {
    icon: 'rotate-ccw',
    title: 'Updates met een vangnet',
    text: 'sudo stacklane update maakt eerst een pg_dump en installeert pas daarna de nieuwe versie uit onze ondertekende pakketbron.',
  },
]

const sources = [
  { title: 'WHMCS', text: 'Klanten met hun wachtwoord, producten, diensten, facturen en tickets' },
  { title: 'Paymenter', text: 'Klanten, producten, diensten, facturen en tickets' },
  { title: 'Pterodactyl', text: 'Gebruikers en hun servers, meteen gekoppeld' },
]

const roadmap = [
  { icon: 'server', title: 'Meer VPS', text: 'Servers, volumes en snapshots op OpenStack en VirtFusion' },
  { icon: 'globe', title: 'Domeinen', text: 'Registreren, DNS-records en nameservers via Openprovider' },
  { icon: 'app-window', title: 'cPanel', text: 'Webhosting op cPanel & WHM, naast DirectAdmin en Plesk' },
  { icon: 'receipt-text', title: 'Meer facturatie', text: 'Pro rata bij upgrades en mislukte betalingen opnieuw proberen' },
]
</script>

<template>
  <div class="sections">
    <section class="screenshot">
      <div class="frame">
        <img src="/panel-dashboard.webp" width="1600" height="801" alt="Het dashboard van Stacklane met actieve tickets en een VPS die binnenkort verloopt" />
      </div>
    </section>

    <section class="block">
      <div class="intro">
        <p class="eyebrow">Overstappen</p>
        <h2>Neem je klanten mee uit WHMCS, Paymenter of Pterodactyl</h2>
        <p>
          Eén commando leest je huidige systeem en zet alles over. Draai het eerst met --dry-run om te zien wat er gebeurt. Een import kun je veilig
          vaker draaien: wat al in Stacklane staat, slaat hij over.
        </p>
      </div>
      <ul class="sources">
        <li v-for="source in sources" :key="source.title">
          <Icon name="arrow-right-left" :size="18" class="reason-icon" />
          <div>
            <p class="roadmap-title">{{ source.title }}</p>
            <p class="roadmap-text">{{ source.text }}</p>
          </div>
        </li>
      </ul>
      <a class="import-link" href="/docs/guides/import">
        Zo werkt importeren
        <Icon name="arrow-right" :size="16" />
      </a>
    </section>

    <section class="block">
      <div class="intro">
        <p class="eyebrow">Zo draait het</p>
        <h2>Installeren met apt, zoals de rest van je server</h2>
        <p>
          Stacklane komt als Debian-pakket uit onze eigen pakketbron op stacklane.io. Je voegt die één keer toe en installeert daarna met apt. Het
          setup-commando vraagt je domein, zet de database klaar, schrijft de Caddy-configuratie en maakt je eerste beheerder aan.
        </p>
      </div>
      <div class="install">
        <div class="terminal">
          <div class="terminal-bar">
            <span /><span /><span />
            <p>Debian 12+ of Ubuntu 24.04+</p>
          </div>
          <pre><code>{{ install }}</code></pre>
        </div>
        <ul class="reasons">
          <li v-for="reason in reasons" :key="reason.title">
            <Icon :name="reason.icon" :size="20" class="reason-icon" />
            <div>
              <h3>{{ reason.title }}</h3>
              <p>{{ reason.text }}</p>
            </div>
          </li>
        </ul>
      </div>
    </section>

    <section id="roadmap" class="block">
      <div class="intro">
        <p class="eyebrow">Roadmap</p>
        <h2>Wat er nog aankomt</h2>
        <p>Deze onderdelen staan nog niet in de huidige versie. Ze komen in de volgende releases, ongeveer in deze volgorde.</p>
      </div>
      <ul class="roadmap">
        <li v-for="item in roadmap" :key="item.title">
          <Icon :name="item.icon" :size="18" class="roadmap-icon" />
          <div>
            <p class="roadmap-title">{{ item.title }}</p>
            <p class="roadmap-text">{{ item.text }}</p>
          </div>
        </li>
      </ul>
    </section>

    <section class="block">
      <div class="demo">
        <div>
          <h2>Kijk rond in de demo</h2>
          <p>Log in met één klik als klant of als beheerder. Elk heel uur zetten we de demo terug, dus je kunt niets kapotmaken.</p>
        </div>
        <a class="demo-button" href="https://demo.stacklane.io/login">
          Naar de demo
          <Icon name="arrow-right" :size="16" />
        </a>
      </div>
    </section>
  </div>
</template>

<style scoped>
.sections {
  margin: 0 auto;
  max-width: 1152px;
  padding: 0 24px 64px;
}

.screenshot {
  margin-top: 16px;
}

.frame {
  overflow: hidden;
  border: 1px solid var(--sl-border);
  border-radius: 14px;
  background: #18181b;
  padding: 6px;
}

.frame img {
  display: block;
  width: 100%;
  height: auto;
  border-radius: 9px;
}

.block {
  margin-top: 112px;
}

.eyebrow {
  margin: 0 0 8px;
  color: var(--vp-c-brand-1);
  font-size: 14px;
  font-weight: 600;
}

h2 {
  margin: 0;
  border: 0;
  padding: 0;
  color: var(--vp-c-text-1);
  font-size: 28px;
  font-weight: 600;
  letter-spacing: -0.02em;
  line-height: 1.25;
}

.intro {
  max-width: 680px;
}

.intro > p:last-child {
  margin: 16px 0 0;
  color: var(--vp-c-text-2);
  font-size: 16px;
  line-height: 1.65;
}

.install {
  display: grid;
  gap: 40px;
  margin-top: 40px;
}

@media (min-width: 960px) {
  .install {
    grid-template-columns: 1.15fr 1fr;
    align-items: start;
  }
}

.terminal {
  overflow: hidden;
  border: 1px solid var(--sl-border);
  border-radius: 12px;
  background: #18181b;
}

.terminal-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 14px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.terminal-bar span {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: #3f3f46;
}

.terminal-bar p {
  margin: 0 0 0 8px;
  color: #a1a1aa;
  font-size: 12px;
}

.terminal pre {
  margin: 0;
  overflow-x: auto;
  padding: 18px 20px;
  color: #e4e4e7;
  font-size: 13px;
  line-height: 1.75;
}

.reasons {
  display: grid;
  gap: 24px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.reasons li {
  display: flex;
  gap: 14px;
}

.reason-icon {
  margin-top: 2px;
  color: var(--vp-c-brand-1);
}

.reasons h3 {
  margin: 0;
  color: var(--vp-c-text-1);
  font-size: 15px;
  font-weight: 600;
}

.reasons p {
  margin: 4px 0 0;
  color: var(--vp-c-text-2);
  font-size: 14px;
  line-height: 1.6;
}

.sources {
  display: grid;
  gap: 16px;
  margin: 32px 0 0;
  padding: 0;
  list-style: none;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
}

.sources li {
  display: flex;
  gap: 12px;
  margin: 0;
  border: 1px solid var(--sl-border);
  border-radius: 12px;
  padding: 20px;
}

.import-link {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-top: 24px;
  font-size: 14px;
  font-weight: 500;
  color: var(--vp-c-text-1);
  text-decoration: none;
}

.roadmap {
  display: grid;
  gap: 12px;
  margin: 32px 0 0;
  padding: 0;
  list-style: none;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
}

.roadmap li {
  display: flex;
  gap: 12px;
  padding: 14px 16px;
  border: 1px dashed var(--sl-border);
  border-radius: 10px;
}

.roadmap-icon {
  margin-top: 1px;
  color: var(--vp-c-text-3);
}

.roadmap-title {
  margin: 0;
  color: var(--vp-c-text-1);
  font-size: 14px;
  font-weight: 500;
}

.roadmap-text {
  margin: 2px 0 0;
  color: var(--vp-c-text-2);
  font-size: 13px;
  line-height: 1.5;
}

.demo {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  padding: 32px;
  border: 1px solid var(--sl-border);
  border-radius: 14px;
  background: var(--sl-surface);
}

.demo p {
  margin: 8px 0 0;
  max-width: 560px;
  color: var(--vp-c-text-2);
}

.demo-button {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 18px;
  border-radius: 999px;
  background: var(--vp-button-brand-bg);
  color: #fff;
  font-size: 14px;
  font-weight: 500;
  text-decoration: none;
}

.demo-button:hover {
  background: var(--vp-button-brand-hover-bg);
}
</style>
