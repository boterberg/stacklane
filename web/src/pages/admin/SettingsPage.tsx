import { useEffect, useState, type FormEvent, type ReactNode } from 'react'

import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Checkbox, Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ApiError } from '../../lib/api'
import {
  usePaymentSettings,
  useSavePaymentSettings,
  useSaveSettings,
  useSettings,
  type BillingSettings,
  type BrandAsset,
  type Company,
  type PaymentSettings,
  type SupportSettings,
  useBrandAsset,
  useSaveBranding,
  useSaveThemeCss,
} from '../../lib/admin'
import { applyTheme, neutralSwatch, presets, type Neutral, type Radius } from '../../lib/brandTheme'
import { cn } from '../../lib/cn'
import { countries } from '../../lib/countries'
import { intlLocale, t } from '../../lib/i18n'
import { useMeta, type Branding } from '../../lib/meta'

function Saved({ show }: { show: boolean }) {
  return show ? <span className="text-sm text-zinc-500 dark:text-white/70">{t('Opgeslagen')}</span> : null
}

function FormCard({ title, description, onSubmit, pending, saved, children }: { title: string; description: string; onSubmit: () => void; pending: boolean; saved: boolean; children: ReactNode }) {
  function submit(event: FormEvent) {
    event.preventDefault()
    onSubmit()
  }
  return (
    <Card title={title} description={description}>
      <form onSubmit={submit} className="space-y-6" noValidate>
        {children}
        <div className="flex items-center justify-end gap-3">
          <Saved show={saved} />
          <Button type="submit" variant="primary" disabled={pending}>
            {t('Opslaan')}
          </Button>
        </div>
      </form>
    </Card>
  )
}

function CompanyForm({ initial }: { initial: Company }) {
  const [company, setCompany] = useState(initial)
  const save = useSaveSettings()
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}
  const field = (key: keyof Company) => ({
    value: company[key] ?? '',
    onChange: (event: { target: { value: string } }) => setCompany({ ...company, [key]: event.target.value }),
  })
  return (
    <FormCard title={t('Bedrijf')} description={t('Deze gegevens staan op elke factuur.')} onSubmit={() => save.mutate({ company })} pending={save.isPending} saved={save.isSuccess}>
      <Field label={t('Bedrijfsnaam')} error={errors.name}>
        {(id) => <Input id={id} invalid={!!errors.name} {...field('name')} />}
      </Field>
      <Field label={t('Adres')} description={t('Op één regel, bijvoorbeeld Kerkstraat 1, 1234 AB Amsterdam.')}>
        {(id) => <Input id={id} {...field('address')} />}
      </Field>
      <div className="grid grid-cols-1 gap-6 sm:grid-cols-2">
        <Field label={t('E-mailadres')}>{(id) => <Input id={id} type="email" {...field('email')} />}</Field>
        <Field label={t('Website')}>{(id) => <Input id={id} {...field('website')} />}</Field>
        <Field label={t('KVK-nummer')}>{(id) => <Input id={id} {...field('coc_number')} />}</Field>
        <Field label={t('Btw-nummer')}>{(id) => <Input id={id} {...field('vat_number')} />}</Field>
        <Field label={t('IBAN')}>{(id) => <Input id={id} {...field('iban')} />}</Field>
        <Field label={t('BIC')}>{(id) => <Input id={id} {...field('bic')} />}</Field>
      </div>
    </FormCard>
  )
}

const currencies = ['EUR', 'USD', 'GBP', 'CHF', 'SEK', 'NOK', 'DKK', 'PLN', 'CZK', 'CAD', 'AUD']

function currencyName(code: string) {
  const name = new Intl.DisplayNames([intlLocale()], { type: 'currency' }).of(code) ?? code
  return `${name} (${code})`
}

function BillingForm({ initial }: { initial: BillingSettings }) {
  const [billing, setBilling] = useState(initial)
  const save = useSaveSettings()
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}
  const number = (key: 'tax_rate_basis_points' | 'payment_term_days' | 'renewal_notice_days' | 'suspend_after_days', scale = 1) => ({
    inputMode: 'numeric' as const,
    value: String(billing[key] / scale),
    onChange: (event: { target: { value: string } }) => setBilling({ ...billing, [key]: Math.round(Number(event.target.value.replace(',', '.')) * scale) || 0 }),
  })
  return (
    <FormCard
      title={t('Facturatie')}
      description={t('Wanneer Stacklane factureert en wat er gebeurt als een klant niet betaalt.')}
      onSubmit={() => save.mutate({ billing })}
      pending={save.isPending}
      saved={save.isSuccess}
    >
      <div className="grid grid-cols-1 gap-6 sm:grid-cols-2">
        <Field label={t('Land van je bedrijf')} error={errors.home_country}>
          {(id) => (
            <Select id={id} value={billing.home_country} onChange={(event) => setBilling({ ...billing, home_country: event.target.value })}>
              {countries().map((country) => (
                <option key={country.code} value={country.code}>
                  {country.name}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t('Valuta')} description={t('Nieuwe facturen gebruiken deze valuta. Bestaande facturen houden hun eigen valuta.')} error={errors.currency}>
          {(id) => (
            <Select id={id} value={billing.currency} onChange={(event) => setBilling({ ...billing, currency: event.target.value })}>
              {currencies.map((code) => (
                <option key={code} value={code}>
                  {currencyName(code)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t('Btw-tarief (%)')} description={t('Je prijzen zijn inclusief dit tarief.')} error={errors.tax_rate_basis_points}>
          {(id) => <Input id={id} {...number('tax_rate_basis_points', 100)} />}
        </Field>
        <div className="sm:col-span-2">
          <Checkbox label={t('OSS-regeling gebruiken')} checked={billing.oss} onChange={(event) => setBilling({ ...billing, oss: event.target.checked })} />
          <p className="mt-1 text-sm text-zinc-500 dark:text-white/60">
            {t('Particulieren in een ander EU-land betalen dan het btw-tarief van hun eigen land. Zakelijke klanten met een geldig EU-btw-nummer krijgen altijd verlegde btw.')}
          </p>
        </div>
        <Field label={t('Betaaltermijn (dagen)')}>{(id) => <Input id={id} {...number('payment_term_days')} />}</Field>
        <Field label={t('Factuur maken (dagen voor verlengen)')}>{(id) => <Input id={id} {...number('renewal_notice_days')} />}</Field>
        <Field label={t('Opschorten na (dagen te laat)')} error={errors.billing}>
          {(id) => <Input id={id} {...number('suspend_after_days')} />}
        </Field>
      </div>
    </FormCard>
  )
}

function PaymentsForm({ initial }: { initial: PaymentSettings }) {
  const [form, setForm] = useState({
    mollie_enabled: initial.mollie_enabled,
    stripe_enabled: initial.stripe_enabled,
    bank_transfer_enabled: initial.bank_transfer_enabled,
    mollie_key: '',
    stripe_secret_key: '',
    stripe_webhook_secret: '',
  })
  const save = useSavePaymentSettings()
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}
  const stored = t('Opgeslagen. Laat leeg om te houden.')
  return (
    <FormCard
      title={t('Betalingen')}
      description={t('Hoe klanten hun facturen betalen. Sleutels worden versleuteld opgeslagen.')}
      onSubmit={() => save.mutate(form, { onSuccess: () => setForm((current) => ({ ...current, mollie_key: '', stripe_secret_key: '', stripe_webhook_secret: '' })) })}
      pending={save.isPending}
      saved={save.isSuccess}
    >
      <div className="space-y-4">
        <Checkbox
          label={t('Mollie: iDEAL, Bancontact, kaart en automatisch incasseren')}
          checked={form.mollie_enabled}
          onChange={(event) => setForm({ ...form, mollie_enabled: event.target.checked })}
        />
        <Field label={t('API-sleutel van Mollie')} description={t('Begint met live_ of test_. De webhook stelt Stacklane zelf in.')} error={errors.mollie_key}>
          {(id) => (
            <Input
              id={id}
              type="password"
              autoComplete="off"
              placeholder={initial.mollie_configured ? stored : 'live_...'}
              invalid={!!errors.mollie_key}
              value={form.mollie_key}
              onChange={(event) => setForm({ ...form, mollie_key: event.target.value })}
            />
          )}
        </Field>
      </div>
      <div className="space-y-4 border-t border-zinc-200 pt-6 dark:border-white/10">
        <Checkbox label={t('Stripe: creditcards en opgeslagen kaarten')} checked={form.stripe_enabled} onChange={(event) => setForm({ ...form, stripe_enabled: event.target.checked })} />
        <Field label={t('Geheime sleutel van Stripe')} error={errors.stripe_secret_key}>
          {(id) => (
            <Input
              id={id}
              type="password"
              autoComplete="off"
              placeholder={initial.stripe_configured ? stored : 'sk_live_...'}
              invalid={!!errors.stripe_secret_key}
              value={form.stripe_secret_key}
              onChange={(event) => setForm({ ...form, stripe_secret_key: event.target.value })}
            />
          )}
        </Field>
        <Field
          label={t('Webhookgeheim van Stripe')}
          description={
            <>
              {t('Maak in Stripe een webhook naar dit adres, met de events checkout.session.completed, checkout.session.expired, payment_intent.succeeded en payment_intent.payment_failed:')}{' '}
              <code className="rounded bg-zinc-100 px-1.5 py-0.5 font-mono text-xs text-zinc-800 dark:bg-white/10 dark:text-white">{initial.webhook_urls.stripe}</code>
            </>
          }
        >
          {(id) => (
            <Input
              id={id}
              type="password"
              autoComplete="off"
              placeholder={initial.stripe_webhook_configured ? stored : 'whsec_...'}
              value={form.stripe_webhook_secret}
              onChange={(event) => setForm({ ...form, stripe_webhook_secret: event.target.value })}
            />
          )}
        </Field>
      </div>
      <div className="border-t border-zinc-200 pt-6 dark:border-white/10">
        <Checkbox
          label={t('Overboeking: toon je IBAN op facturen en bij het betalen')}
          checked={form.bank_transfer_enabled}
          onChange={(event) => setForm({ ...form, bank_transfer_enabled: event.target.checked })}
        />
      </div>
    </FormCard>
  )
}

function SupportForm({ initial }: { initial: SupportSettings }) {
  const [days, setDays] = useState(String(initial.auto_close_days))
  const save = useSaveSettings()
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}
  return (
    <FormCard
      title={t('Support')}
      description={t('Klanten krijgen een melding en een e-mail als support op hun ticket reageert.')}
      onSubmit={() => save.mutate({ support: { auto_close_days: Math.round(Number(days)) || 0 } })}
      pending={save.isPending}
      saved={save.isSuccess}
    >
      <Field
        label={t('Tickets sluiten na (dagen zonder reactie)')}
        description={t('Een beantwoord ticket gaat dicht als de klant zo lang niets laat horen. Met 0 blijven tickets open.')}
        error={errors.auto_close_days}
      >
        {(id) => <Input id={id} inputMode="numeric" invalid={!!errors.auto_close_days} value={days} onChange={(event) => setDays(event.target.value)} />}
      </Field>
    </FormCard>
  )
}

function readFile(file: File) {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(String(reader.result))
    reader.onerror = () => reject(reader.error)
    reader.readAsDataURL(file)
  })
}

function AssetRow({ kind, label, description, version, accept }: { kind: BrandAsset; label: string; description: string; version: number | null; accept: string }) {
  const { upload, remove } = useBrandAsset(kind)
  const error = [upload.error, remove.error].find((value) => value instanceof ApiError) as ApiError | undefined
  const busy = upload.isPending || remove.isPending
  return (
    <div className="flex flex-col gap-4 border-t border-zinc-200 pt-6 first:border-t-0 first:pt-0 sm:flex-row sm:items-center dark:border-white/10">
      <div className="flex h-14 w-full shrink-0 items-center justify-center rounded-lg border border-dashed border-zinc-300 bg-white sm:w-40 dark:border-white/20 dark:bg-zinc-900">
        {version ? (
          <img src={`/api/branding/${kind}?v=${version}`} alt="" className="max-h-10 max-w-36 object-contain" />
        ) : (
          <span className="text-xs text-zinc-400 dark:text-white/40">{t('Standaard')}</span>
        )}
      </div>
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium text-zinc-800 dark:text-white">{label}</p>
        <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{description}</p>
        {error && <p className="mt-1 text-sm text-red-600 dark:text-red-400">{error.fieldErrors.file ?? error.message}</p>}
      </div>
      <div className="flex shrink-0 gap-2">
        <label className="inline-flex h-8 cursor-pointer items-center rounded-md border border-zinc-200 bg-white px-3 text-sm font-medium text-zinc-800 shadow-xs transition-colors hover:bg-zinc-50 dark:border-zinc-600 dark:bg-zinc-700 dark:text-white dark:hover:bg-zinc-600/75">
          {t('Kies bestand')}
          <input
            type="file"
            accept={accept}
            className="sr-only"
            disabled={busy}
            onChange={async (event) => {
              const file = event.target.files?.[0]
              event.target.value = ''
              if (file) upload.mutate(await readFile(file))
            }}
          />
        </label>
        {version && (
          <Button size="sm" variant="ghost" disabled={busy} onClick={() => remove.mutate()}>
            {t('Verwijderen')}
          </Button>
        )}
      </div>
    </div>
  )
}

function ColorField({ label, description, value, onChange, error, fallback }: { label: string; description: string; value: string; onChange: (value: string) => void; error?: string; fallback: string }) {
  return (
    <Field label={label} description={description} error={error}>
      {(id) => (
        <div className="flex gap-2">
          <input
            type="color"
            aria-label={t('Kies een kleur')}
            value={/^#[0-9a-f]{6}$/i.test(value) ? value : fallback}
            onChange={(event) => onChange(event.target.value)}
            className="h-10 w-12 shrink-0 cursor-pointer rounded-lg border border-zinc-200 bg-white p-1 dark:border-white/10 dark:bg-white/5"
          />
          <Input id={id} placeholder={fallback} invalid={!!error} value={value} onChange={(event) => onChange(event.target.value)} />
        </div>
      )}
    </Field>
  )
}

function neutralLabel(neutral: Neutral) {
  switch (neutral) {
    case 'zinc':
      return t('Grijs')
    case 'slate':
      return t('Leisteen')
    case 'gray':
      return t('Koel grijs')
    case 'neutral':
      return t('Neutraal')
    case 'stone':
      return t('Warm grijs')
    case 'tinted':
      return t('Getint met accent')
  }
}

function radiusLabel(radius: Radius) {
  switch (radius) {
    case 'sharp':
      return t('Strak')
    case 'default':
      return t('Standaard')
    case 'round':
      return t('Rond')
  }
}

function presetLabel(name: string) {
  switch (name) {
    case 'Indigo':
      return t('Indigo')
    case 'Oceaan':
      return t('Oceaan')
    case 'Bos':
      return t('Bos')
    case 'Zonsondergang':
      return t('Zonsondergang')
    case 'Framboos':
      return t('Framboos')
    default:
      return t('Lavendel')
  }
}

const neutrals: Neutral[] = ['zinc', 'slate', 'gray', 'neutral', 'stone', 'tinted']
const corners: Radius[] = ['sharp', 'default', 'round']

function choiceClass(selected: boolean) {
  return cn(
    'flex items-center gap-3 rounded-lg border px-3 py-2.5 text-left text-sm font-medium transition-colors',
    selected
      ? 'accent-border border-zinc-800 bg-zinc-800/5 text-zinc-800 dark:border-white dark:bg-white/10 dark:text-white'
      : 'border-zinc-200 text-zinc-600 hover:border-zinc-300 hover:bg-zinc-50 dark:border-white/10 dark:text-white/80 dark:hover:bg-white/5',
  )
}

function validColor(value: string) {
  return /^#[0-9a-f]{6}$/i.test(value.trim()) ? value.trim() : null
}

function BrandingForm({ initial }: { initial: Branding }) {
  const [name, setName] = useState(initial.name ?? '')
  const [accent, setAccent] = useState(initial.accent ?? '')
  const [accentDark, setAccentDark] = useState(initial.accent_dark ?? '')
  const [neutral, setNeutral] = useState<Neutral>((initial.neutral as Neutral | null) ?? 'zinc')
  const [radius, setRadius] = useState<Radius>((initial.radius as Radius | null) ?? 'default')
  const save = useSaveBranding()
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}
  useEffect(() => {
    applyTheme({ accent: validColor(accent), accent_dark: validColor(accentDark), neutral, radius, css: initial.css })
  }, [accent, accentDark, neutral, radius, initial.css])

  useEffect(() => () => applyTheme(initial), [initial])

  function choosePreset(preset: (typeof presets)[number]) {
    setAccent(preset.accent)
    setAccentDark(preset.accent_dark)
    setNeutral(preset.neutral)
  }

  function reset() {
    setAccent('')
    setAccentDark('')
    setNeutral('zinc')
    setRadius('default')
  }

  return (
    <FormCard
      title={t('Huisstijl en thema')}
      description={t('Zet je eigen naam, kleuren en vormen op het paneel. Je ziet elke wijziging meteen, maar klanten pas na opslaan.')}
      onSubmit={() => save.mutate({ name, accent, accent_dark: accentDark, neutral, radius })}
      pending={save.isPending}
      saved={save.isSuccess}
    >
      <div className="space-y-8">
        <Field label={t('Naam van het paneel')} description={t('Staat in de titel van het tabblad, en in de zijbalk als je geen logo hebt.')} error={errors.name}>
          {(id) => <Input id={id} placeholder="Stacklane" invalid={!!errors.name} value={name} onChange={(event) => setName(event.target.value)} />}
        </Field>

        <div>
          <p className="text-sm font-medium text-zinc-800 dark:text-white">{t('Kant-en-klare thema\'s')}</p>
          <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{t('Een goed begin. Je kunt daarna alles nog aanpassen.')}</p>
          <div className="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-4">
            <button type="button" onClick={reset} className={choiceClass(!accent && neutral === 'zinc' && radius === 'default')}>
              <span className="flex shrink-0 -space-x-1">
                <span className="size-4 rounded-full bg-zinc-800 ring-2 ring-white dark:bg-white dark:ring-zinc-900" />
                <span className="size-4 rounded-full bg-zinc-300 ring-2 ring-white dark:ring-zinc-900" />
              </span>
              {t('Standaard')}
            </button>
            {presets.map((preset) => (
              <button key={preset.name} type="button" onClick={() => choosePreset(preset)} className={choiceClass(accent.toLowerCase() === preset.accent && neutral === preset.neutral)}>
                <span className="flex shrink-0 -space-x-1">
                  <span className="size-4 rounded-full ring-2 ring-white dark:ring-zinc-900" style={{ backgroundColor: preset.accent }} />
                  <span className="size-4 rounded-full ring-2 ring-white dark:ring-zinc-900" style={{ backgroundColor: neutralSwatch(preset.neutral, preset.accent)[1] }} />
                </span>
                <span className="truncate">{presetLabel(preset.name)}</span>
              </button>
            ))}
          </div>
        </div>

        <div className="grid grid-cols-1 gap-6 sm:grid-cols-2">
          <ColorField label={t('Accentkleur')} description={t('Voor knoppen, tabbladen, het actieve menu-item en grafieken. Laat leeg voor zwart-wit.')} value={accent} onChange={setAccent} error={errors.accent} fallback="#4f46e5" />
          <ColorField label={t('Accentkleur in donkere modus')} description={t('Optioneel. Een lichtere tint leest beter op een donkere achtergrond.')} value={accentDark} onChange={setAccentDark} error={errors.accent_dark} fallback="#818cf8" />
        </div>

        <div>
          <p className="text-sm font-medium text-zinc-800 dark:text-white">{t('Achtergrond en tekst')}</p>
          <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{t('De grijstinten van het hele paneel, in lichte en donkere modus.')}</p>
          <div className="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-3">
            {neutrals.map((option) => (
              <button key={option} type="button" aria-pressed={neutral === option} onClick={() => setNeutral(option)} className={choiceClass(neutral === option)}>
                <span className="flex shrink-0 overflow-hidden rounded-md ring-1 ring-zinc-800/10 dark:ring-white/10">
                  {neutralSwatch(option, validColor(accent)).map((swatch) => (
                    <span key={swatch} className="h-6 w-3" style={{ backgroundColor: swatch }} />
                  ))}
                </span>
                <span className="truncate">{neutralLabel(option)}</span>
              </button>
            ))}
          </div>
          {errors.neutral && <p className="mt-2 text-sm text-red-600 dark:text-red-400">{errors.neutral}</p>}
        </div>

        <div>
          <p className="text-sm font-medium text-zinc-800 dark:text-white">{t('Hoeken')}</p>
          <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{t('Hoe rond knoppen, velden en kaarten zijn.')}</p>
          <div className="mt-3 grid grid-cols-3 gap-2">
            {corners.map((option) => (
              <button key={option} type="button" aria-pressed={radius === option} onClick={() => setRadius(option)} className={choiceClass(radius === option)}>
                <span
                  className="size-5 shrink-0 border-2 border-b-0 border-l-0 border-current opacity-60"
                  style={{ borderTopRightRadius: option === 'sharp' ? 2 : option === 'round' ? 12 : 6 }}
                />
                {radiusLabel(option)}
              </button>
            ))}
          </div>
        </div>
      </div>
    </FormCard>
  )
}

function CssForm({ initial }: { initial: string | null }) {
  const [css, setCss] = useState('')
  const [loaded, setLoaded] = useState(!initial)
  const save = useSaveThemeCss()
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}

  useEffect(() => {
    if (!initial) return
    let active = true
    fetch(`/api/branding/theme.css?v=${initial}`)
      .then((response) => (response.ok ? response.text() : ''))
      .then((text) => {
        if (!active) return
        setCss(text)
        setLoaded(true)
      })
    return () => {
      active = false
    }
  }, [initial])

  return (
    <FormCard
      title={t('Eigen CSS')}
      description={t('Voor een volledig eigen thema. Deze CSS laadt na de standaardstijl, op elke pagina en ook op de inlogpagina.')}
      onSubmit={() => save.mutate(css)}
      pending={save.isPending || !loaded}
      saved={save.isSuccess}
    >
      <Field label={t('CSS')} description={t('Hoogstens 64 kB. Laat leeg om de eigen CSS uit te zetten. Je kunt de kleurvariabelen gebruiken, zoals var(--accent).')} error={errors.css}>
        {(id) => (
          <textarea
            id={id}
            rows={10}
            spellCheck={false}
            value={css}
            disabled={!loaded}
            onChange={(event) => setCss(event.target.value)}
            placeholder={'.accent-fill {\n  border-radius: 9999px;\n}'}
            className="accent-focus block w-full rounded-lg border border-zinc-200 bg-white px-3 py-2 font-mono text-xs text-zinc-700 shadow-xs placeholder:text-zinc-400 focus:outline-2 focus:-outline-offset-1 focus:outline-zinc-800/15 dark:border-white/10 dark:bg-white/10 dark:text-zinc-300 dark:focus:outline-white/20"
          />
        )}
      </Field>
    </FormCard>
  )
}

function BrandAssets({ branding }: { branding: Branding }) {
  return (
    <Card title={t('Logo en favicon')} description={t('PNG, JPG of WebP, hoogstens 512 kB. Wijzigingen staan direct live.')}>
      <div className="space-y-6">
        <AssetRow
          kind="logo"
          label={t('Logo')}
          description={t('Voor een lichte achtergrond. Liefst breed, zoals 600 × 120 pixels.')}
          version={branding.logo}
          accept="image/png,image/jpeg,image/webp"
        />
        <AssetRow
          kind="logo_dark"
          label={t('Logo voor donkere modus')}
          description={t('Optioneel. Zonder dit bestand gebruiken we het gewone logo.')}
          version={branding.logo_dark}
          accept="image/png,image/jpeg,image/webp"
        />
        <AssetRow
          kind="favicon"
          label={t('Favicon')}
          description={t('Het icoon in het tabblad. Vierkant, zoals 64 × 64 pixels. ICO mag ook.')}
          version={branding.favicon}
          accept="image/png,image/jpeg,image/webp,image/x-icon,.ico"
        />
      </div>
    </Card>
  )
}

export function SettingsPage() {
  const { data: settings } = useSettings()
  const { data: payments } = usePaymentSettings()
  const { data: meta } = useMeta()
  return (
    <>
      <PageHeader title={t('Instellingen')} />
      <div className="max-w-3xl space-y-6">
        {settings && <CompanyForm initial={settings.company} />}
        {meta && <BrandingForm initial={meta.branding} />}
        {meta && <BrandAssets branding={meta.branding} />}
        {meta && <CssForm initial={meta.branding.css} />}
        {settings && <BillingForm initial={settings.billing} />}
        {settings && <SupportForm initial={settings.support} />}
        {payments && <PaymentsForm initial={payments} />}
      </div>
    </>
  )
}
