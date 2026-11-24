import { Popover, PopoverButton, PopoverPanel } from '@headlessui/react'
import {
  ArrowUturnLeftIcon,
  ArrowUturnRightIcon,
  BoldIcon,
  CodeBracketIcon,
  ItalicIcon,
  LinkIcon,
  ListBulletIcon,
  NumberedListIcon,
  StrikethroughIcon,
  UnderlineIcon,
  H2Icon,
  ChatBubbleBottomCenterTextIcon,
} from '@heroicons/react/24/outline'
import Highlight from '@tiptap/extension-highlight'
import { EditorContent, useEditor, useEditorState, type Editor } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import { useEffect, useState, type ComponentType, type SVGProps } from 'react'

import { cn } from '../../lib/cn'
import { t } from '../../lib/i18n'

type RichTextEditorProps = {
  id?: string
  value: string
  onChange: (html: string) => void
  invalid?: boolean
  minHeight?: string
}

function ToolbarButton({
  label,
  icon: Icon,
  active,
  disabled,
  onClick,
}: {
  label: string
  icon: ComponentType<SVGProps<SVGSVGElement>>
  active?: boolean
  disabled?: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      aria-pressed={active}
      disabled={disabled}
      onMouseDown={(event) => event.preventDefault()}
      onClick={onClick}
      className={cn(
        'flex size-8 items-center justify-center rounded-md text-zinc-500 transition-colors hover:bg-zinc-800/5 hover:text-zinc-800 disabled:opacity-40 dark:text-white/60 dark:hover:bg-white/10 dark:hover:text-white',
        active && 'bg-zinc-800/10 text-zinc-800 dark:bg-white/15 dark:text-white',
      )}
    >
      <Icon className="size-[18px]" aria-hidden />
    </button>
  )
}

function Divider() {
  return <span className="mx-1 h-5 w-px bg-zinc-200 dark:bg-white/15" />
}

function LinkButton({ editor }: { editor: Editor }) {
  const [url, setUrl] = useState('')
  const active = editor.isActive('link')

  if (active) {
    return <ToolbarButton label={t('Link verwijderen')} icon={LinkIcon} active onClick={() => editor.chain().focus().unsetLink().run()} />
  }

  return (
    <Popover className="relative">
      <PopoverButton
        title={t('Link')}
        aria-label={t('Link')}
        onMouseDown={(event) => event.preventDefault()}
        className="flex size-8 items-center justify-center rounded-md text-zinc-500 transition-colors hover:bg-zinc-800/5 hover:text-zinc-800 dark:text-white/60 dark:hover:bg-white/10 dark:hover:text-white"
      >
        <LinkIcon className="size-[18px]" aria-hidden />
      </PopoverButton>
      <PopoverPanel anchor={{ to: 'bottom start', gap: 4 }} className="z-50 w-72 rounded-lg border border-zinc-200 bg-white p-2 shadow-lg dark:border-zinc-600 dark:bg-zinc-700">
        {({ close }) => (
          <form
            className="flex gap-2"
            onSubmit={(event) => {
              event.preventDefault()
              const href = url.trim()
              if (href) {
                editor.chain().focus().extendMarkRange('link').setLink({ href: /^(https?:|mailto:)/.test(href) ? href : `https://${href}` }).run()
              }
              setUrl('')
              close()
            }}
          >
            <input
              autoFocus
              value={url}
              onChange={(event) => setUrl(event.target.value)}
              placeholder="https://"
              className="h-8 min-w-0 flex-1 rounded-md border border-zinc-200 bg-white px-2 text-sm dark:border-white/10 dark:bg-white/10 dark:text-white"
            />
            <button type="submit" className="h-8 rounded-md bg-zinc-800 px-3 text-sm font-medium text-white dark:bg-white dark:text-zinc-800">
              {t('Toevoegen')}
            </button>
          </form>
        )}
      </PopoverPanel>
    </Popover>
  )
}

function Toolbar({ editor }: { editor: Editor }) {
  const state = useEditorState({
    editor,
    selector: ({ editor: current }) => ({
      bold: current.isActive('bold'),
      italic: current.isActive('italic'),
      strike: current.isActive('strike'),
      underline: current.isActive('underline'),
      heading: current.isActive('heading', { level: 2 }),
      bulletList: current.isActive('bulletList'),
      orderedList: current.isActive('orderedList'),
      blockquote: current.isActive('blockquote'),
      code: current.isActive('code'),
      canUndo: current.can().undo(),
      canRedo: current.can().redo(),
    }),
  })

  return (
    <div className="flex flex-wrap items-center gap-0.5 border-b border-zinc-200 px-2 py-1.5 dark:border-white/10">
      <ToolbarButton label={t('Kop')} icon={H2Icon} active={state.heading} onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()} />
      <Divider />
      <ToolbarButton label={t('Vet')} icon={BoldIcon} active={state.bold} onClick={() => editor.chain().focus().toggleBold().run()} />
      <ToolbarButton label={t('Cursief')} icon={ItalicIcon} active={state.italic} onClick={() => editor.chain().focus().toggleItalic().run()} />
      <ToolbarButton label={t('Doorhalen')} icon={StrikethroughIcon} active={state.strike} onClick={() => editor.chain().focus().toggleStrike().run()} />
      <ToolbarButton label={t('Onderstrepen')} icon={UnderlineIcon} active={state.underline} onClick={() => editor.chain().focus().toggleUnderline().run()} />
      <Divider />
      <ToolbarButton label={t('Opsomming')} icon={ListBulletIcon} active={state.bulletList} onClick={() => editor.chain().focus().toggleBulletList().run()} />
      <ToolbarButton label={t('Genummerde lijst')} icon={NumberedListIcon} active={state.orderedList} onClick={() => editor.chain().focus().toggleOrderedList().run()} />
      <ToolbarButton label={t('Citaat')} icon={ChatBubbleBottomCenterTextIcon} active={state.blockquote} onClick={() => editor.chain().focus().toggleBlockquote().run()} />
      <Divider />
      <LinkButton editor={editor} />
      <ToolbarButton label={t('Code')} icon={CodeBracketIcon} active={state.code} onClick={() => editor.chain().focus().toggleCode().run()} />
      <div className="ml-auto flex">
        <ToolbarButton label={t('Ongedaan maken')} icon={ArrowUturnLeftIcon} disabled={!state.canUndo} onClick={() => editor.chain().focus().undo().run()} />
        <ToolbarButton label={t('Opnieuw')} icon={ArrowUturnRightIcon} disabled={!state.canRedo} onClick={() => editor.chain().focus().redo().run()} />
      </div>
    </div>
  )
}

export function RichTextEditor({ id, value, onChange, invalid, minHeight = 'min-h-60' }: RichTextEditorProps) {
  const editor = useEditor({
    extensions: [StarterKit.configure({ link: { openOnClick: false, autolink: true } }), Highlight],
    content: value,
    editorProps: {
      attributes: {
        ...(id ? { id } : {}),
        class: cn('rich-text px-4 py-3 text-sm text-zinc-800 focus:outline-none dark:text-zinc-200', minHeight),
      },
    },
    onUpdate: ({ editor: current }) => onChange(current.isEmpty ? '' : current.getHTML()),
  })

  useEffect(() => {
    if (editor && value === '' && !editor.isEmpty) {
      editor.commands.clearContent()
    }
  }, [editor, value])

  return (
    <div
      className={cn(
        'overflow-hidden rounded-lg border border-zinc-200 border-b-zinc-300/80 bg-white shadow-xs focus-within:outline-2 focus-within:-outline-offset-1 focus-within:outline-zinc-800/15 dark:border-white/10 dark:bg-white/10 dark:shadow-none dark:focus-within:outline-white/20',
        invalid && 'border-red-500 dark:border-red-400',
      )}
    >
      {editor && <Toolbar editor={editor} />}
      <EditorContent editor={editor} />
    </div>
  )
}
