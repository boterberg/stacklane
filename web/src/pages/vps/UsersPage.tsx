import { PencilSquareIcon, PlusIcon, ShareIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'

import { SlideOver } from '../../components/layout/SlideOver'
import { Badge } from '../../components/ui/Badge'
import { Button, ButtonLink } from '../../components/ui/Button'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { DataTable } from '../../components/ui/ResourceList'
import { ApiError } from '../../lib/api'
import { formatDate } from '../../lib/format'
import { t } from '../../lib/i18n'
import { permissionGroups, useCan, useMembers, useRemoveMember, useSaveMember, type Member } from '../../lib/vpsAccess'

const allRights = permissionGroups.flatMap((group) => group.rights.map(([right]) => `${group.id}.${right}`))

function Checkbox({ checked, indeterminate, disabled, onChange, label }: { checked: boolean; indeterminate?: boolean; disabled?: boolean; onChange: () => void; label: string }) {
  return (
    <label className={`inline-flex items-center gap-2 text-sm ${disabled ? 'text-zinc-400 dark:text-white/40' : 'text-zinc-800 dark:text-white'}`}>
      <input
        type="checkbox"
        className="size-4 rounded accent-zinc-800 dark:accent-white"
        checked={checked}
        disabled={disabled}
        ref={(element) => {
          if (element) element.indeterminate = !!indeterminate
        }}
        onChange={onChange}
      />
      {label}
    </label>
  )
}

function MemberPanel({ open, member, onClose }: { open: boolean; member: Member | null; onClose: () => void }) {
  const can = useCan()
  const save = useSaveMember()
  const [email, setEmail] = useState('')
  const [chosen, setChosen] = useState<Set<string>>(new Set())
  const [loadedFor, setLoadedFor] = useState<string | null | undefined>(undefined)
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}
  const grantable = allRights.filter(can)

  if (open && loadedFor !== (member?.id ?? null)) {
    setLoadedFor(member?.id ?? null)
    setEmail(member?.email ?? '')
    setChosen(new Set(member?.permissions ?? []))
  }

  function close() {
    save.reset()
    setLoadedFor(undefined)
    onClose()
  }

  function toggle(rights: string[]) {
    const next = new Set(chosen)
    const on = rights.every((right) => next.has(right))
    rights.forEach((right) => (on ? next.delete(right) : next.add(right)))
    setChosen(next)
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    save.mutate({ id: member?.id, email, permissions: [...chosen] }, { onSuccess: close })
  }

  return (
    <SlideOver
      open={open}
      onClose={close}
      title={member ? t('Rechten aanpassen') : t('Gebruiker toevoegen')}
      description={member ? member.email : t('Geef iemand met een eigen account toegang tot je VPS-omgeving.')}
      footer={
        <>
          <Button onClick={close}>{t('Annuleren')}</Button>
          <Button variant="primary" type="submit" form="vps-member" disabled={save.isPending}>
            {save.isPending ? t('Bezig...') : member ? t('Opslaan') : t('Toevoegen')}
          </Button>
        </>
      }
    >
      <form id="vps-member" onSubmit={submit} className="space-y-8" noValidate>
        {!member && (
          <Field label={t('E-mailadres')} error={errors.email}>
            {(id) => <Input id={id} type="email" autoComplete="off" invalid={!!errors.email} value={email} onChange={(event) => setEmail(event.target.value)} />}
          </Field>
        )}
        <div>
          <div className="flex items-center justify-between gap-4">
            <p className="text-sm font-medium text-zinc-800 dark:text-white">{t('Rechten')}</p>
            <Checkbox
              label={t('Alles selecteren')}
              checked={grantable.length > 0 && grantable.every((right) => chosen.has(right))}
              indeterminate={chosen.size > 0 && !grantable.every((right) => chosen.has(right))}
              onChange={() => toggle(grantable)}
            />
          </div>
          {errors.permissions && <p className="mt-2 text-sm text-red-500 dark:text-red-400">{errors.permissions}</p>}
          <div className="mt-4 space-y-3">
            {permissionGroups.map((group) => {
              const rights = group.rights.map(([right]) => `${group.id}.${right}`)
              const usable = rights.filter(can)
              const count = rights.filter((right) => chosen.has(right)).length
              return (
                <fieldset key={group.id} className="rounded-xl border border-zinc-200 px-5 py-4 dark:border-white/10">
                  <legend className="sr-only">{t(group.label)}</legend>
                  <div className="flex items-center justify-between gap-4">
                    <Checkbox
                      label={t(group.label)}
                      checked={usable.length > 0 && usable.every((right) => chosen.has(right))}
                      indeterminate={count > 0 && !usable.every((right) => chosen.has(right))}
                      disabled={usable.length === 0}
                      onChange={() => toggle(usable)}
                    />
                    <span className="text-sm text-zinc-500 dark:text-white/60">
                      {count}/{rights.length}
                    </span>
                  </div>
                  <div className="mt-3 grid gap-2 pl-6 sm:grid-cols-2">
                    {group.rights.map(([right, label]) => {
                      const key = `${group.id}.${right}`
                      return <Checkbox key={key} label={t(label)} checked={chosen.has(key)} disabled={!can(key)} onChange={() => toggle([key])} />
                    })}
                  </div>
                </fieldset>
              )
            })}
          </div>
        </div>
      </form>
    </SlideOver>
  )
}

export function UsersPage() {
  const can = useCan()
  const allowed = can('access.users')
  const { data, isPending } = useMembers(allowed)
  const remove = useRemoveMember()
  const [editing, setEditing] = useState<Member | null>(null)
  const [adding, setAdding] = useState(false)
  const [confirm, setConfirm] = useState<Member | null>(null)
  const members = data ?? []

  return (
    <>
      <PageHeader
        title={t('Gebruikers')}
        description={t('Geef anderen toegang tot je VPS-omgeving en kies per persoon wat ze mogen.')}
        actions={
          <div className="flex gap-2">
            <ButtonLink size="sm" icon={ShareIcon} to="/account/shared-access">
              {t('Gedeelde toegang')}
            </ButtonLink>
            {allowed && (
              <Button size="sm" icon={PlusIcon} onClick={() => setAdding(true)}>
                {t('Gebruiker toevoegen')}
              </Button>
            )}
          </div>
        }
      />
      {!allowed ? (
        <EmptyState title={t('Geen toegang')} description={t('Je mag de gebruikers van deze omgeving niet beheren.')} />
      ) : !isPending && members.length === 0 ? (
        <EmptyState title={t('Geen gebruikers')} description={t('Je VPS-omgeving heeft nog geen gebruikers met toegang.')} />
      ) : (
        <DataTable<Member>
          rows={members}
          rowKey={(member) => member.id}
          columns={[
            {
              header: t('Gebruiker'),
              cell: (member) => (
                <div>
                  <p className="flex items-center gap-2 font-medium text-zinc-800 dark:text-white">
                    {member.name}
                    {!member.accepted && <Badge color="amber">{t('Uitgenodigd')}</Badge>}
                  </p>
                  <p className="text-zinc-500 dark:text-white/60">{member.email}</p>
                </div>
              ),
            },
            { header: t('Rechten'), cell: (member) => t('{count} van {total}', { count: member.permissions.length, total: allRights.length }) },
            { header: t('Toegevoegd'), cell: (member) => formatDate(member.created_at) },
            {
              header: '',
              className: 'text-right',
              cell: (member) => (
                <div className="flex justify-end gap-2">
                  <Button size="sm" icon={PencilSquareIcon} onClick={() => setEditing(member)}>
                    {t('Rechten')}
                  </Button>
                  <Button size="sm" icon={TrashIcon} onClick={() => setConfirm(member)}>
                    {t('Verwijderen')}
                  </Button>
                </div>
              ),
            },
          ]}
        />
      )}
      <MemberPanel open={adding || editing !== null} member={editing} onClose={() => (setAdding(false), setEditing(null))} />
      <ConfirmDialog
        open={confirm !== null}
        title={t('Toegang intrekken?')}
        confirmLabel={t('Intrekken')}
        danger
        pending={remove.isPending}
        onClose={() => setConfirm(null)}
        onConfirm={() => confirm && remove.mutate(confirm.id, { onSettled: () => setConfirm(null) })}
      >
        {t('{name} kan daarna niet meer bij je VPS-omgeving.', { name: confirm?.name ?? '' })}
      </ConfirmDialog>
    </>
  )
}
