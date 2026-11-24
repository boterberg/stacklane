import { RouterProvider } from 'react-router'

import { useLocale } from '../lib/i18n'
import { router } from './router'

export function App() {
  const locale = useLocale()
  return <RouterProvider key={locale} router={router} />
}
