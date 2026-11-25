import { PlusIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'

import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Field, Input, Select } from '../../components/ui/Field'
import { t } from '../../lib/i18n'
import { emptyRule, isManaged, ruleNote, type NewRule } from '../../lib/firewall'
import type { FirewallRule } from '../../lib/vps'

export function RuleForm({ onAdd, pending, errors }: { onAdd: (rule: NewRule, done: () => void) => void; pending: boolean; errors: Record<string, string> }) {
  const [rule, setRule] = useState(emptyRule)
  return (
    <form
      noValidate
      className="grid grid-cols-1 gap-4 sm:grid-cols-3"
      onSubmit={(event) => {
        event.preventDefault()
        onAdd(rule, () => setRule(emptyRule))
      }}
    >
      <Field label={t('Richting')}>
        {(id) => (
          <Select id={id} value={rule.type} onChange={(event) => setRule({ ...rule, type: event.target.value as 'in' | 'out' })}>
            <option value="in">{t('Inkomend')}</option>
            <option value="out">{t('Uitgaand')}</option>
          </Select>
        )}
      </Field>
      <Field label={t('Actie')} error={errors.action}>
        {(id) => (
          <Select id={id} value={rule.action} onChange={(event) => setRule({ ...rule, action: event.target.value })}>
            <option value="ACCEPT">{t('Toestaan')}</option>
            <option value="DROP">{t('Blokkeren')}</option>
          </Select>
        )}
      </Field>
      <Field label={t('Protocol')} error={errors.proto}>
        {(id) => (
          <Select id={id} value={rule.proto} onChange={(event) => setRule({ ...rule, proto: event.target.value })}>
            <option value="tcp">TCP</option>
            <option value="udp">UDP</option>
            <option value="icmp">ICMP</option>
          </Select>
        )}
      </Field>
      <Field label={t('Poort')} error={errors.dport}>
        {(id) => <Input id={id} placeholder="22" invalid={!!errors.dport} value={rule.dport} onChange={(event) => setRule({ ...rule, dport: event.target.value })} />}
      </Field>
      <Field label={t('Vanaf IP-adres')} error={errors.source}>
        {(id) => <Input id={id} placeholder={t('Iedereen')} invalid={!!errors.source} value={rule.source} onChange={(event) => setRule({ ...rule, source: event.target.value })} />}
      </Field>
      <Field label={t('Notitie')}>{(id) => <Input id={id} value={rule.comment} onChange={(event) => setRule({ ...rule, comment: event.target.value })} />}</Field>
      <div className="flex justify-end sm:col-span-3">
        <Button type="submit" variant="primary" icon={PlusIcon} disabled={pending}>
          {t('Regel toevoegen')}
        </Button>
      </div>
    </form>
  )
}

export function RuleList({
  rules,
  onDelete,
  pending,
  canDelete = () => true,
}: {
  rules: FirewallRule[]
  onDelete: (rule: FirewallRule) => void
  pending: boolean
  canDelete?: (rule: FirewallRule) => boolean
}) {
  return (
    <ul className="divide-y divide-zinc-200 rounded-xl border border-zinc-200 bg-white dark:divide-white/10 dark:border-white/10 dark:bg-white/10">
      {rules.map((item) => (
        <li key={item.id} className="flex flex-wrap items-center justify-between gap-4 px-6 py-4 text-sm">
          <div className="flex flex-wrap items-center gap-3">
            <Badge color={item.action === 'ACCEPT' ? 'green' : 'red'}>{item.action === 'ACCEPT' ? t('Toestaan') : t('Blokkeren')}</Badge>
            <span className="text-zinc-800 dark:text-white">
              {item.type === 'in' ? t('Inkomend') : t('Uitgaand')} · {(item.proto ?? t('alles')).toUpperCase()}
              {item.dport ? ` · ${t('poort {port}', { port: item.dport })}` : ''}
            </span>
            <span className="text-zinc-500 dark:text-white/60">{item.source ? t('vanaf {source}', { source: item.source }) : t('vanaf iedereen')}</span>
            {ruleNote(item.comment) && <span className="text-zinc-400 dark:text-white/50">{ruleNote(item.comment)}</span>}
            {isManaged(item) && <Badge>{t('Via een firewall')}</Badge>}
          </div>
          {canDelete(item) && (
            <Button size="sm" icon={TrashIcon} disabled={pending} onClick={() => onDelete(item)}>
              {t('Verwijderen')}
            </Button>
          )}
        </li>
      ))}
    </ul>
  )
}
