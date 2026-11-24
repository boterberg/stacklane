import { ExclamationTriangleIcon, ListBulletIcon, ServerStackIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { useNavigate, useParams } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { Button, ButtonLink } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { EmptyState } from '../../components/ui/EmptyState'
import { Checkbox, Field, Input } from '../../components/ui/Field'
import { ApiError } from '../../lib/api'
import { useDeleteFirewall, useFirewallDetail, useFirewallServers, useSaveFirewall, type FirewallDetail, type FirewallInput, type SaveResult } from '../../lib/firewall'
import { t } from '../../lib/i18n'
import type { FirewallRule } from '../../lib/vps'
import { RuleForm, RuleList } from './FirewallRules'

function inputOf(firewall: FirewallDetail): FirewallInput {
  return {
    name: firewall.name,
    auto_assign: firewall.auto_assign,
    rules: firewall.rule_list.map(({ type, action, proto, dport, source, comment }) => ({ type, action, proto, dport, source, comment })),
  }
}

function Problems({ result }: { result: SaveResult | undefined }) {
  if (!result?.problems.length) return null
  return (
    <div className="mb-6 flex gap-3 rounded-xl border border-amber-200 bg-amber-50 px-6 py-4 text-sm text-amber-800 dark:border-amber-400/20 dark:bg-amber-400/10 dark:text-amber-300">
      <ExclamationTriangleIcon className="size-5 shrink-0" aria-hidden />
      <div>
        <p className="font-medium">{t('Opgeslagen, maar niet elke server is bijgewerkt:')}</p>
        <ul className="mt-1 space-y-1">
          {result.problems.map((problem) => (
            <li key={problem.server}>
              {problem.server}: {problem.error}
            </li>
          ))}
        </ul>
      </div>
    </div>
  )
}

function Settings({ firewall, save }: { firewall: FirewallDetail; save: (input: FirewallInput) => void }) {
  const [name, setName] = useState(firewall.name)
  const [autoAssign, setAutoAssign] = useState(firewall.auto_assign)
  return (
    <Card title={t('Instellingen')}>
      <form
        noValidate
        className="space-y-6"
        onSubmit={(event) => {
          event.preventDefault()
          save({ ...inputOf(firewall), name, auto_assign: autoAssign })
        }}
      >
        <Field label={t('Naam')}>{(id) => <Input id={id} value={name} onChange={(event) => setName(event.target.value)} />}</Field>
        <div>
          <Checkbox label={t('Automatisch toewijzen')} checked={autoAssign} onChange={(event) => setAutoAssign(event.target.checked)} />
          <p className="mt-1 text-sm text-zinc-500 dark:text-white/60">{t('Nieuwe servers krijgen deze firewall vanzelf.')}</p>
        </div>
        <div className="flex justify-end">
          <Button type="submit" variant="primary">
            {t('Opslaan')}
          </Button>
        </div>
      </form>
    </Card>
  )
}

function Servers({ firewall, save, pending }: { firewall: FirewallDetail; save: (input: FirewallInput) => void; pending: boolean }) {
  const { data: servers = [] } = useFirewallServers()
  const [chosen, setChosen] = useState(() => new Set(firewall.server_list.map((server) => server.id)))
  function toggle(id: string, on: boolean) {
    const next = new Set(chosen)
    if (on) next.add(id)
    else next.delete(id)
    setChosen(next)
  }
  return (
    <Card title={t('Servers')} description={t('De regels van deze firewall komen bovenaan de firewall van elke gekoppelde server.')}>
      {servers.length === 0 ? (
        <p className="text-sm text-zinc-500 dark:text-white/60">{t('Je hebt nog geen server die een firewall ondersteunt.')}</p>
      ) : (
        <form
          noValidate
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault()
            save({ ...inputOf(firewall), server_ids: [...chosen] })
          }}
        >
          {servers.map((server) => (
            <div key={server.id}>
              <Checkbox label={server.name} checked={chosen.has(server.id)} onChange={(event) => toggle(server.id, event.target.checked)} />
            </div>
          ))}
          <div className="flex justify-end">
            <Button type="submit" variant="primary" disabled={pending}>
              {t('Opslaan')}
            </Button>
          </div>
        </form>
      )}
    </Card>
  )
}

export function FirewallPage() {
  const { id = '' } = useParams()
  const { data: firewall, isPending, error } = useFirewallDetail(id)
  const save = useSaveFirewall()
  const remove = useDeleteFirewall()
  const navigate = useNavigate()
  const [confirm, setConfirm] = useState(false)
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}

  if (isPending) return null
  if (error || !firewall) {
    return (
      <EmptyState
        title={t('Firewall niet gevonden')}
        description={t('Deze firewall bestaat niet of is van iemand anders.')}
        action={
          <ButtonLink to="/vps/firewalls" size="sm">
            {t('Naar firewalls')}
          </ButtonLink>
        }
      />
    )
  }

  const store = (input: FirewallInput, done?: () => void) => save.mutate({ id, input }, { onSuccess: done })
  const rules = firewall.rule_list
  const without = (rule: FirewallRule) => ({ ...inputOf(firewall), rules: inputOf(firewall).rules.filter((_, index) => rules[index].id !== rule.id) })

  return (
    <>
      <div className="mb-6 flex flex-wrap items-start justify-between gap-4">
        <div>
          <div className="flex flex-wrap items-center gap-3">
            <h1 className="text-xl font-medium text-zinc-800 dark:text-white">{firewall.name}</h1>
            {firewall.auto_assign && <Badge color="blue">{t('Automatisch toewijzen')}</Badge>}
          </div>
          <div className="mt-3 flex flex-wrap items-center gap-x-6 gap-y-1 text-sm text-zinc-500 dark:text-white/70">
            <span className="flex items-center gap-2">
              <ListBulletIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {firewall.rules === 1 ? t('1 firewallregel') : t('{count} firewallregels', { count: firewall.rules })}
            </span>
            <span className="flex items-center gap-2">
              <ServerStackIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {firewall.servers === 1 ? t('1 server') : t('{count} servers', { count: firewall.servers })}
            </span>
          </div>
        </div>
        <Button size="sm" icon={TrashIcon} onClick={() => setConfirm(true)}>
          {t('Verwijderen')}
        </Button>
      </div>

      <Problems result={save.data ?? remove.data} />
      {errors.name && <p className="mb-6 text-sm font-medium text-red-500 dark:text-red-400">{errors.name}</p>}

      <div className="max-w-4xl space-y-6">
        <Card title={t('Regel toevoegen')} description={t('Zonder regels is al het verkeer toegestaan. Regels gelden van boven naar beneden.')}>
          <RuleForm pending={save.isPending} errors={errors} onAdd={(rule, done) => store({ ...inputOf(firewall), rules: [...inputOf(firewall).rules, rule] }, done)} />
          {errors.rules && <p className="mt-3 text-sm font-medium text-red-500 dark:text-red-400">{errors.rules}</p>}
        </Card>
        {rules.length === 0 ? (
          <EmptyState title={t('Geen regels')} description={t('Deze firewall laat nog al het verkeer door.')} />
        ) : (
          <RuleList rules={rules} pending={save.isPending} onDelete={(rule) => store(without(rule))} />
        )}
        <Servers key={firewall.server_list.map((server) => server.id).join()} firewall={firewall} pending={save.isPending} save={(input) => store(input)} />
        <Settings key={`${firewall.name}-${firewall.auto_assign}`} firewall={firewall} save={(input) => store(input)} />
      </div>

      <ConfirmDialog
        open={confirm}
        title={t('Firewall verwijderen?')}
        confirmLabel={t('Verwijderen')}
        danger
        pending={remove.isPending}
        onClose={() => setConfirm(false)}
        onConfirm={() => remove.mutate(id, { onSuccess: () => navigate('/vps/firewalls') })}
      >
        {t('De regels van deze firewall verdwijnen van alle gekoppelde servers.')}
      </ConfirmDialog>
    </>
  )
}
