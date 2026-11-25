import { ArrowRightIcon, ArrowUpRightIcon } from '@heroicons/react/16/solid'
import { Link } from 'react-router'

type LinkCardProps = {
  href: string
  title: string
  description: string
}

export function LinkCard({ href, title, description }: LinkCardProps) {
  const className =
    'group relative block rounded-lg border border-zinc-200 bg-white p-4 transition-[background-color,translate,box-shadow] duration-200 ease-out hover:bg-zinc-50 hover:shadow-md hover:shadow-zinc-900/5 motion-safe:hover:-translate-y-0.5 dark:border-white/10 dark:bg-white/10 dark:hover:bg-white/15 dark:hover:shadow-black/20'
  const internal = href.startsWith('/')
  const Icon = internal ? ArrowRightIcon : ArrowUpRightIcon
  const content = (
    <>
      <Icon
        className="absolute top-4 right-4 size-4 text-zinc-400 transition-[color,translate] duration-200 group-hover:text-zinc-800 motion-safe:group-hover:translate-x-0.5 dark:text-white/50 dark:group-hover:text-white"
        aria-hidden
      />
      <h3 className="pr-6 text-sm leading-6 font-medium text-zinc-800 dark:text-white">{title}</h3>
      <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{description}</p>
    </>
  )
  return internal ? (
    <Link to={href} className={className}>
      {content}
    </Link>
  ) : (
    <a href={href} target="_blank" rel="noreferrer" className={className}>
      {content}
    </a>
  )
}
