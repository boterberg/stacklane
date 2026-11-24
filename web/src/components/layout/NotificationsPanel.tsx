import { Link } from 'react-router'

import { useNotifications } from '../../lib/notifications'
import { EmptyState } from '../ui/EmptyState'
import { SlideOver } from './SlideOver'

export function NotificationsPanel({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { data } = useNotifications()
  const count = data?.unread_count ?? 0

  return (
    <SlideOver
      open={open}
      onClose={onClose}
      title="Meldingen"
      description={count === 1 ? 'Je hebt 1 ongelezen melding.' : `Je hebt ${count} ongelezen meldingen.`}
    >
      {data && data.items.length > 0 ? (
        <ul className="divide-y divide-zinc-200 dark:divide-white/10">
          {data.items.map((item) => (
            <li key={item.id} className="py-4">
              <p className="text-sm font-medium text-zinc-800 dark:text-white">{item.title}</p>
              <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{item.body}</p>
              {item.link && (
                <Link to={item.link} onClick={onClose} className="mt-2 inline-block text-sm font-medium text-zinc-800 underline underline-offset-4 dark:text-white">
                  Bekijken
                </Link>
              )}
            </li>
          ))}
        </ul>
      ) : (
        <EmptyState title="Geen meldingen" description="Je hebt geen ongelezen meldingen." />
      )}
    </SlideOver>
  )
}
