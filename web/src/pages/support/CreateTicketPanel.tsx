import { InboxIcon, QuestionMarkCircleIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router'

import { SlideOver } from '../../components/layout/SlideOver'
import { Button } from '../../components/ui/Button'
import { Field, Input, Select } from '../../components/ui/Field'
import { RichTextEditor } from '../../components/ui/RichTextEditor'
import { ApiError } from '../../lib/api'
import { useCreateTicket, useDepartments } from '../../lib/support'
import { t } from '../../lib/i18n'

const emptyForm = { department_id: '', subject: '', body_html: '' }

export function CreateTicketPanel({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [form, setForm] = useState(emptyForm)
  const { data: departments = [] } = useDepartments()
  const create = useCreateTicket()
  const navigate = useNavigate()
  const errors = create.error instanceof ApiError ? create.error.fieldErrors : {}

  function close() {
    onClose()
    create.reset()
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    create.mutate(
      { ...form, department_id: form.department_id || null },
      {
        onSuccess: (ticket) => {
          setForm(emptyForm)
          close()
          navigate(`/support/tickets/${ticket.number}`)
        },
      },
    )
  }

  return (
    <SlideOver
      open={open}
      onClose={close}
      title={t('Een ticket maken')}
      description={t('Maak een nieuw ticket aan.')}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            {t('Annuleren')}
          </Button>
          <Button variant="primary" type="submit" form="create-ticket" disabled={create.isPending}>
            {t('Maken')}
          </Button>
        </>
      }
    >
      <form id="create-ticket" onSubmit={submit} className="space-y-6" noValidate>
        <Field label={t('Afdeling')} error={errors.department_id}>
          {(id) => (
            <div className="relative">
              <InboxIcon className="pointer-events-none absolute top-3 left-3 size-4 text-zinc-400 dark:text-white/60" aria-hidden />
              <Select
                id={id}
                value={form.department_id}
                onChange={(event) => setForm({ ...form, department_id: event.target.value })}
                className={form.department_id ? 'pl-9' : 'pl-9 text-zinc-400 dark:text-zinc-400'}
              >
                <option value="">{t('Selecteer een afdeling...')}</option>
                {departments.map((department) => (
                  <option key={department.id} value={department.id}>
                    {department.name}
                  </option>
                ))}
              </Select>
            </div>
          )}
        </Field>
        <Field label={t('Onderwerp')} description={t('Vat je vraag kort samen, details kun je hieronder kwijt.')} error={errors.subject}>
          {(id) => (
            <div className="relative">
              <QuestionMarkCircleIcon className="pointer-events-none absolute top-3 left-3 size-4 text-zinc-400 dark:text-white/60" aria-hidden />
              <Input id={id} className="pl-9" maxLength={150} invalid={!!errors.subject} value={form.subject} onChange={(event) => setForm({ ...form, subject: event.target.value })} />
            </div>
          )}
        </Field>
        <Field label={t('Bericht')} error={errors.body_html}>
          {(id) => <RichTextEditor id={id} value={form.body_html} invalid={!!errors.body_html} onChange={(body_html) => setForm((current) => ({ ...current, body_html }))} minHeight="min-h-72" />}
        </Field>
      </form>
    </SlideOver>
  )
}
