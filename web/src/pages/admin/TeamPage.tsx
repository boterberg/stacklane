import { useState } from 'react'

import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { Checkbox, Field, FormError, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { useTeam, useTeamChanges, type MemberGrant, type StaffPermission, type TeamMember } from '../../lib/admin'
import { ApiError } from '../../lib/api'
import { useMe } from '../../lib/auth'
import { formatRelative } from '../../lib/format'
import { t } from '../../lib/i18n'

const permissions: StaffPermission[] = ['support', 'billing', 'customers', 'services']

function permissionLabel(permission: StaffPermission) {
  switch (permission) {
    case 'support':
      return t('Support')
    case 'billing':
      return t('Facturatie')
    case 'customers':
      return t('Klanten')
    case 'services':
      return t('Diensten en producten')
  }
}

function permissionHint(permission: StaffPermission) {
  switch (permission) {
    case 'support':
      return t('Tickets beantwoorden en de kennisbank bijhouden.')
    case 'billing':
      return t('Facturen, betalingen, btw-overzicht en statistieken.')
    case 'customers':
      return t('Klanten bekijken met hun diensten en facturen.')
    case 'services':
      return t('Diensten aanmaken en beheren, producten en domeinextensies.')
  }
}

function GrantFields({ value, onChange, error }: { value: MemberGrant; onChange: (value: MemberGrant) => void; error?: string }) {
  return (
    <div className="space-y-4">
      <Field label={t('Rol')} description={value.role === 'admin' ? t('Mag alles, ook instellingen, koppelingen en het team.') : t('Mag alleen wat je hieronder aanvinkt.')}>
        {(id) => (
          <Select id={id} value={value.role} onChange={(event) => onChange({ ...value, role: event.target.value as MemberGrant['role'] })}>
            <option value="staff">{t('Medewerker')}</option>
            <option value="admin">{t('Beheerder')}</option>
          </Select>
        )}
      </Field>
      {value.role === 'staff' && (
        <fieldset className="space-y-3">
          <legend className="text-sm font-medium text-zinc-800 dark:text-white">{t('Rechten')}</legend>
          {permissions.map((permission) => (
            <div key={permission}>
              <Checkbox
                label={permissionLabel(permission)}
                checked={value.permissions.includes(permission)}
                onChange={(event) =>
                  onChange({
                    ...value,
                    permissions: event.target.checked ? [...value.permissions, permission] : value.permissions.filter((granted) => granted !== permission),
                  })
                }
              />
              <p className="mt-0.5 pl-7 text-sm text-zinc-500 dark:text-white/70">{permissionHint(permission)}</p>
            </div>
          ))}
          {error && <p className="text-sm text-red-600 dark:text-red-400">{error}</p>}
        </fieldset>
      )}
    </div>
  )
}

function AddMember() {
  const [email, setEmail] = useState('')
  const [grant, setGrant] = useState<MemberGrant>({ role: 'staff', permissions: ['support'] })
  const { add } = useTeamChanges()
  const errors = add.error instanceof ApiError ? add.error.fieldErrors : {}
  return (
    <Card title={t('Iemand toevoegen')} description={t('Diegene maakt eerst zelf een account aan. Daarna voeg je dat account hier toe.')}>
      <form
        className="space-y-6"
        onSubmit={(event) => {
          event.preventDefault()
          add.mutate({ email, ...grant }, { onSuccess: () => setEmail('') })
        }}
      >
        <Field label={t('E-mailadres')} error={errors.email}>
          {(id) => <Input id={id} type="email" required invalid={!!errors.email} value={email} onChange={(event) => setEmail(event.target.value)} />}
        </Field>
        <GrantFields value={grant} onChange={setGrant} error={errors.permissions} />
        {!errors.email && !errors.permissions && <FormError error={add.error} />}
        <div className="flex justify-end">
          <Button type="submit" variant="primary" disabled={add.isPending}>
            {t('Toevoegen')}
          </Button>
        </div>
      </form>
    </Card>
  )
}

function MemberRow({ member, self }: { member: TeamMember; self: boolean }) {
  const [editing, setEditing] = useState(false)
  const [confirming, setConfirming] = useState(false)
  const [grant, setGrant] = useState<MemberGrant>({ role: member.role, permissions: member.permissions })
  const { update, remove } = useTeamChanges()
  const errors = update.error instanceof ApiError ? update.error.fieldErrors : {}
  const name = `${member.first_name} ${member.last_name}`.trim() || member.email
  return (
    <li className="py-4 first:pt-0 last:pb-0">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <p className="truncate text-sm font-medium text-zinc-800 dark:text-white">{name}</p>
            {self && <Badge>{t('Jij')}</Badge>}
            {!member.two_factor && <Badge color="amber">{t('Geen 2FA')}</Badge>}
          </div>
          <p className="mt-0.5 truncate text-sm text-zinc-500 dark:text-white/70">{member.email}</p>
          <div className="mt-2 flex flex-wrap gap-1.5">
            {member.role === 'admin' ? (
              <Badge color="sky">{t('Beheerder')}</Badge>
            ) : (
              member.permissions.map((permission) => (
                <Badge key={permission} color="zinc">
                  {permissionLabel(permission)}
                </Badge>
              ))
            )}
          </div>
          <p className="mt-2 text-xs text-zinc-400 dark:text-white/50">
            {member.last_active_at ? t('Laatst actief {time}', { time: formatRelative(member.last_active_at) }) : t('Nog niet ingelogd')}
          </p>
        </div>
        {!self && (
          <div className="flex shrink-0 gap-2">
            <Button size="sm" onClick={() => setEditing(!editing)}>
              {editing ? t('Sluiten') : t('Rechten wijzigen')}
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setConfirming(true)}>
              {t('Uit team halen')}
            </Button>
          </div>
        )}
      </div>
      {editing && (
        <form
          className="mt-4 space-y-4 rounded-lg border border-zinc-200 bg-white p-4 dark:border-white/10 dark:bg-white/5"
          onSubmit={(event) => {
            event.preventDefault()
            update.mutate({ id: member.id, ...grant }, { onSuccess: () => setEditing(false) })
          }}
        >
          <GrantFields value={grant} onChange={setGrant} error={errors.permissions} />
          {!errors.permissions && <FormError error={update.error} />}
          <div className="flex justify-end">
            <Button type="submit" variant="primary" size="sm" disabled={update.isPending}>
              {t('Opslaan')}
            </Button>
          </div>
        </form>
      )}
      <ConfirmDialog
        open={confirming}
        title={t('{name} uit het team halen?', { name })}
        confirmLabel={t('Uit team halen')}
        danger
        pending={remove.isPending}
        onClose={() => setConfirming(false)}
        onConfirm={() => remove.mutate(member.id, { onSuccess: () => setConfirming(false) })}
      >
        {t('Het account blijft bestaan als gewoon klantaccount. Diegene kan meteen niet meer bij het beheer, en API-tokens met beheerrechten worden ingetrokken.')}
      </ConfirmDialog>
    </li>
  )
}

export function TeamPage() {
  const { data: team } = useTeam()
  const { data: me } = useMe()
  return (
    <>
      <PageHeader title={t('Team')} description={t('Geef medewerkers toegang tot alleen de delen van het beheer die ze nodig hebben.')} />
      <div className="max-w-3xl space-y-6">
        <Card title={t('Teamleden')}>
          {team && (
            <ul className="divide-y divide-zinc-200 dark:divide-white/10">
              {team.map((member) => (
                <MemberRow key={member.id} member={member} self={member.id === me?.id} />
              ))}
            </ul>
          )}
        </Card>
        <AddMember />
      </div>
    </>
  )
}
