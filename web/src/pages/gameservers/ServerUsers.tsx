import { useState } from 'react'

import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { Checkbox, Field, FormError, Input } from '../../components/ui/Field'
import { ApiError } from '../../lib/api'
import { gamePermissionHint, gamePermissionLabel, gamePermissions, useGameMemberChanges, useGameMembers, type GameMember, type GamePermission } from '../../lib/gameservers'
import { t } from '../../lib/i18n'

function PermissionChecks({ value, onChange, error }: { value: GamePermission[]; onChange: (value: GamePermission[]) => void; error?: string }) {
  return (
    <fieldset className="space-y-3">
      <legend className="text-sm font-medium text-zinc-800 dark:text-white">{t('Rechten')}</legend>
      {gamePermissions.map((permission) => (
        <div key={permission}>
          <Checkbox
            label={gamePermissionLabel(permission)}
            checked={value.includes(permission)}
            onChange={(event) => onChange(event.target.checked ? [...value, permission] : value.filter((granted) => granted !== permission))}
          />
          <p className="mt-0.5 pl-7 text-sm text-zinc-500 dark:text-white/70">{gamePermissionHint(permission)}</p>
        </div>
      ))}
      {error && <p className="text-sm text-red-600 dark:text-red-400">{error}</p>}
    </fieldset>
  )
}

function MemberRow({ serverId, member }: { serverId: string; member: GameMember }) {
  const [editing, setEditing] = useState(false)
  const [confirming, setConfirming] = useState(false)
  const [permissions, setPermissions] = useState<GamePermission[]>(member.permissions)
  const { update, remove } = useGameMemberChanges(serverId)
  const errors = update.error instanceof ApiError ? update.error.fieldErrors : {}
  return (
    <li className="py-4 first:pt-0 last:pb-0">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <p className="truncate text-sm font-medium text-zinc-800 dark:text-white">{member.name}</p>
            {!member.accepted && <Badge color="amber">{t('Uitgenodigd')}</Badge>}
          </div>
          <p className="mt-0.5 truncate text-sm text-zinc-500 dark:text-white/70">{member.email}</p>
          <div className="mt-2 flex flex-wrap gap-1.5">
            {member.permissions.map((permission) => (
              <Badge key={permission}>{gamePermissionLabel(permission)}</Badge>
            ))}
          </div>
        </div>
        <div className="flex shrink-0 gap-2">
          <Button size="sm" onClick={() => setEditing(!editing)}>
            {editing ? t('Sluiten') : t('Rechten wijzigen')}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setConfirming(true)}>
            {member.accepted ? t('Toegang intrekken') : t('Uitnodiging intrekken')}
          </Button>
        </div>
      </div>
      {editing && (
        <form
          className="mt-4 space-y-4 rounded-lg border border-zinc-200 bg-white p-4 dark:border-white/10 dark:bg-white/5"
          onSubmit={(event) => {
            event.preventDefault()
            update.mutate({ member: member.id, permissions }, { onSuccess: () => setEditing(false) })
          }}
        >
          <PermissionChecks value={permissions} onChange={setPermissions} error={errors.permissions} />
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
        title={t('Toegang van {name} intrekken?', { name: member.name })}
        confirmLabel={t('Intrekken')}
        danger
        pending={remove.isPending}
        onClose={() => setConfirming(false)}
        onConfirm={() => remove.mutate(member.id, { onSuccess: () => setConfirming(false) })}
      >
        {t('Diegene kan meteen niet meer bij deze server. Wat al in de activiteit staat, blijft staan.')}
      </ConfirmDialog>
    </li>
  )
}

function InviteForm({ serverId }: { serverId: string }) {
  const [email, setEmail] = useState('')
  const [permissions, setPermissions] = useState<GamePermission[]>(['console', 'power'])
  const [sent, setSent] = useState(false)
  const { invite } = useGameMemberChanges(serverId)
  const errors = invite.error instanceof ApiError ? invite.error.fieldErrors : {}
  return (
    <Card title={t('Iemand uitnodigen')} description={t('Diegene heeft een account nodig met een bevestigd e-mailadres, en accepteert de uitnodiging onder Gedeelde toegang.')}>
      <form
        className="space-y-6"
        onSubmit={(event) => {
          event.preventDefault()
          setSent(false)
          invite.mutate(
            { email, permissions },
            {
              onSuccess: () => {
                setEmail('')
                setSent(true)
              },
            },
          )
        }}
      >
        <Field label={t('E-mailadres')} error={errors.email}>
          {(id) => <Input id={id} type="email" required invalid={!!errors.email} value={email} onChange={(event) => setEmail(event.target.value)} />}
        </Field>
        <PermissionChecks value={permissions} onChange={setPermissions} error={errors.permissions} />
        {!errors.email && !errors.permissions && <FormError error={invite.error} />}
        <div className="flex items-center justify-end gap-3">
          {sent && <span className="text-sm text-zinc-500 dark:text-white/70">{t('Uitnodiging verstuurd.')}</span>}
          <Button type="submit" variant="primary" disabled={invite.isPending}>
            {t('Uitnodigen')}
          </Button>
        </div>
      </form>
    </Card>
  )
}

export function ServerUsers({ serverId }: { serverId: string }) {
  const { data: members } = useGameMembers(serverId)
  return (
    <div className="max-w-3xl space-y-6">
      <Card title={t('Gebruikers met toegang')} description={t('Alleen jij kunt gebruikers toevoegen en rechten wijzigen.')}>
        {members && members.length === 0 && <p className="text-sm text-zinc-500 dark:text-white/70">{t('Nog niemand. Nodig hieronder iemand uit.')}</p>}
        {members && members.length > 0 && (
          <ul className="divide-y divide-zinc-200 dark:divide-white/10">
            {members.map((member) => (
              <MemberRow key={member.id} serverId={serverId} member={member} />
            ))}
          </ul>
        )}
      </Card>
      <InviteForm serverId={serverId} />
    </div>
  )
}
