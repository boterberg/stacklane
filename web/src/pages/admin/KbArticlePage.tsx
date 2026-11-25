import { ArrowTopRightOnSquareIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'
import { useNavigate, useParams } from 'react-router'

import { Button, ButtonLink } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { Checkbox, Field, FormError, Input } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { RichTextEditor } from '../../components/ui/RichTextEditor'
import { ApiError } from '../../lib/api'
import { t } from '../../lib/i18n'
import { useAdminArticle, useDeleteArticle, useSaveArticle, type Article, type ArticleInput } from '../../lib/kb'

const empty: ArticleInput = { title: '', category: '', body_html: '', published: false }

export function KbArticlePage() {
  const { id: param = 'new' } = useParams()
  const id = param === 'new' ? null : param
  const { data: article, isPending } = useAdminArticle(id)
  if (id && (isPending || !article)) return null
  return <ArticleForm key={id ?? 'new'} id={id} article={article ?? null} />
}

function ArticleForm({ id, article }: { id: string | null; article: Article | null }) {
  const navigate = useNavigate()
  const save = useSaveArticle(id)
  const remove = useDeleteArticle()
  const [form, setForm] = useState<ArticleInput>(article ? { title: article.title, category: article.category, body_html: article.body_html, published: article.published } : empty)
  const [confirm, setConfirm] = useState(false)
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}

  function submit(event: FormEvent) {
    event.preventDefault()
    save.mutate(form, { onSuccess: (saved) => !id && navigate(`/admin/kb/${saved.id}`, { replace: true }) })
  }

  return (
    <>
      <PageHeader
        title={id ? t('Artikel bewerken') : t('Artikel schrijven')}
        actions={
          id && (
            <>
              {article?.published && (
                <ButtonLink to={`/help/${article.slug}`} size="sm" icon={ArrowTopRightOnSquareIcon}>
                  {t('Bekijken')}
                </ButtonLink>
              )}
              <Button size="sm" icon={TrashIcon} onClick={() => setConfirm(true)}>
                {t('Verwijderen')}
              </Button>
            </>
          )
        }
      />
      <form onSubmit={submit} noValidate className="max-w-4xl">
        <Card title={t('Artikel')}>
          <div className="space-y-6">
            <div className="grid grid-cols-1 gap-6 sm:grid-cols-[2fr_1fr]">
              <Field label={t('Titel')} error={errors.title}>
                {(field) => <Input id={field} invalid={!!errors.title} value={form.title} onChange={(event) => setForm({ ...form, title: event.target.value })} />}
              </Field>
              <Field label={t('Categorie')} error={errors.category}>
                {(field) => (
                  <Input id={field} placeholder={t('Bijvoorbeeld VPS')} invalid={!!errors.category} value={form.category} onChange={(event) => setForm({ ...form, category: event.target.value })} />
                )}
              </Field>
            </div>
            <Field label={t('Tekst')} error={errors.body_html}>
              {(field) => (
                <RichTextEditor id={field} value={form.body_html} invalid={!!errors.body_html} onChange={(body_html) => setForm((current) => ({ ...current, body_html }))} minHeight="min-h-96" />
              )}
            </Field>
            <Checkbox label={t('Publiceren in het helpcentrum')} checked={form.published} onChange={(event) => setForm({ ...form, published: event.target.checked })} />
            <FormError error={save.error} />
            <div className="flex items-center justify-end gap-3">
              {save.isSuccess && <span className="text-sm text-zinc-500 dark:text-white/70">{t('Opgeslagen')}</span>}
              <Button type="submit" variant="primary" disabled={save.isPending}>
                {t('Opslaan')}
              </Button>
            </div>
          </div>
        </Card>
      </form>
      <ConfirmDialog
        open={confirm}
        title={t('Artikel verwijderen?')}
        confirmLabel={t('Verwijderen')}
        danger
        pending={remove.isPending}
        onClose={() => setConfirm(false)}
        onConfirm={() => id && remove.mutate(id, { onSuccess: () => navigate('/admin/kb', { replace: true }) })}
      >
        {t('Klanten kunnen dit artikel daarna niet meer lezen. Dit kun je niet ongedaan maken.')}
      </ConfirmDialog>
    </>
  )
}
