import { useMeta } from '../../lib/meta'
import { cn } from '../../lib/cn'
import { Logo } from './Logo'

export function BrandLogo({ className }: { className?: string }) {
  const { data: meta } = useMeta()
  const branding = meta?.branding
  if (!meta) return <span className={cn('block', className)} />
  if (branding?.logo) {
    const name = branding.name ?? 'Logo'
    return (
      <>
        <img src={`/api/branding/logo?v=${branding.logo}`} alt={name} className={cn('w-auto object-contain', className, !!branding.logo_dark && 'dark:hidden')} />
        {!!branding.logo_dark && <img src={`/api/branding/logo_dark?v=${branding.logo_dark}`} alt={name} className={cn('hidden w-auto object-contain dark:block', className)} />}
      </>
    )
  }
  if (branding?.name) {
    return <span className={cn('flex items-center text-lg leading-none font-semibold tracking-tight text-zinc-900 dark:text-white', className)}>{branding.name}</span>
  }
  return <Logo className={cn('text-zinc-900 dark:text-white', className)} />
}
