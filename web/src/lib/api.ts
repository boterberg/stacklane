import { getLocale, t } from './i18n'

export type FieldErrors = Record<string, string>

export class ApiError extends Error {
  readonly status: number
  readonly fieldErrors: FieldErrors

  constructor(status: number, message: string, fieldErrors: FieldErrors = {}) {
    super(message)
    this.status = status
    this.fieldErrors = fieldErrors
  }
}

type RequestOptions = {
  method?: 'GET' | 'POST' | 'PATCH' | 'PUT' | 'DELETE'
  body?: unknown
}

const OWNER_KEY = 'stacklane.vps-owner'

function readOwner(): string | null {
  try {
    return window.localStorage.getItem(OWNER_KEY)
  } catch {
    return null
  }
}

let vpsOwner = readOwner()

export function currentVpsOwner() {
  return vpsOwner
}

export function setVpsOwner(owner: string | null) {
  vpsOwner = owner
  try {
    if (owner) window.localStorage.setItem(OWNER_KEY, owner)
    else window.localStorage.removeItem(OWNER_KEY)
  } catch {
    return
  }
}

function headersFor(path: string, json: boolean): Record<string, string> {
  const headers: Record<string, string> = { 'accept-language': getLocale() }
  if (json) headers['content-type'] = 'application/json'
  if (vpsOwner && (path.startsWith('/vps/') || path.startsWith('/ssh-keys'))) headers['x-vps-owner'] = vpsOwner
  return headers
}

export async function api<T>(path: string, { method = 'GET', body }: RequestOptions = {}): Promise<T> {
  const response = await fetch(`/api${path}`, {
    method,
    credentials: 'include',
    headers: headersFor(path, body !== undefined),
    body: body === undefined ? undefined : JSON.stringify(body),
  })

  if (response.status === 204 || response.status === 202) {
    return undefined as T
  }

  const payload = await response.json().catch(() => ({}))
  if (!response.ok) {
    throw new ApiError(response.status, payload.message ?? t('Er ging iets mis.'), payload.errors ?? {})
  }
  return payload as T
}
