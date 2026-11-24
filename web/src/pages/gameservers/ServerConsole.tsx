import '@xterm/xterm/css/xterm.css'

import { FitAddon } from '@xterm/addon-fit'
import { Terminal } from '@xterm/xterm'
import { useEffect, useRef, useState, type FormEvent } from 'react'

import { consoleToken } from '../../lib/gameservers'
import { t } from '../../lib/i18n'

type ConnectionState = 'connecting' | 'connected' | 'closed'

export function ServerConsole({ serverId }: { serverId: string }) {
  const container = useRef<HTMLDivElement>(null)
  const socket = useRef<WebSocket | null>(null)
  const [state, setState] = useState<ConnectionState>('connecting')
  const [command, setCommand] = useState('')
  const [history, setHistory] = useState<string[]>([])
  const [historyIndex, setHistoryIndex] = useState(-1)

  useEffect(() => {
    if (!container.current) return
    const terminal = new Terminal({
      convertEol: true,
      disableStdin: true,
      fontSize: 13,
      fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace',
      scrollback: 5000,
      theme: { background: '#18181b', foreground: '#e4e4e7', cursor: '#18181b' },
    })
    const fit = new FitAddon()
    terminal.loadAddon(fit)
    terminal.open(container.current)
    fit.fit()
    const resize = new ResizeObserver(() => fit.fit())
    resize.observe(container.current)

    let stopped = false
    let retry: ReturnType<typeof setTimeout> | undefined

    async function authenticate(target: WebSocket) {
      const { token } = await consoleToken(serverId)
      target.send(JSON.stringify({ event: 'auth', args: [token] }))
    }

    async function connect() {
      setState('connecting')
      try {
        const { token, socket: url } = await consoleToken(serverId)
        if (stopped) return
        const connection = new WebSocket(url)
        socket.current = connection
        connection.onopen = () => connection.send(JSON.stringify({ event: 'auth', args: [token] }))
        connection.onmessage = (message) => {
          const { event, args } = JSON.parse(message.data) as { event: string; args?: string[] }
          if (event === 'auth success') {
            setState('connected')
            connection.send(JSON.stringify({ event: 'send logs', args: [null] }))
          } else if (event === 'console output' || event === 'install output') {
            terminal.writeln(args?.[0] ?? '')
          } else if (event === 'daemon message') {
            terminal.writeln(`\x1b[33m[Stacklane] ${args?.[0] ?? ''}\x1b[0m`)
          } else if (event === 'token expiring') {
            authenticate(connection).catch(() => connection.close())
          } else if (event === 'token expired' || event === 'jwt error') {
            connection.close()
          }
        }
        connection.onclose = () => {
          setState('closed')
          if (!stopped) retry = setTimeout(connect, 3000)
        }
      } catch {
        setState('closed')
        if (!stopped) retry = setTimeout(connect, 5000)
      }
    }

    connect()
    return () => {
      stopped = true
      clearTimeout(retry)
      resize.disconnect()
      socket.current?.close()
      terminal.dispose()
    }
  }, [serverId])

  function send(event: FormEvent) {
    event.preventDefault()
    const value = command.trim()
    if (!value || socket.current?.readyState !== WebSocket.OPEN) return
    socket.current.send(JSON.stringify({ event: 'send command', args: [value] }))
    setHistory((current) => [value, ...current].slice(0, 50))
    setHistoryIndex(-1)
    setCommand('')
  }

  function browseHistory(direction: 1 | -1) {
    const next = Math.min(Math.max(historyIndex + direction, -1), history.length - 1)
    setHistoryIndex(next)
    setCommand(next === -1 ? '' : history[next])
  }

  return (
    <div className="overflow-hidden rounded-xl border border-zinc-200 dark:border-white/10">
      <div className="h-[420px] bg-[#18181b] px-3 pt-3">
        <div ref={container} className="h-full" />
      </div>
      <form onSubmit={send} className="flex items-center gap-3 border-t border-white/10 bg-[#18181b] px-4 py-3">
        <span className="font-mono text-sm text-zinc-500">&gt;</span>
        <input
          value={command}
          onChange={(event) => setCommand(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'ArrowUp') {
              event.preventDefault()
              browseHistory(1)
            } else if (event.key === 'ArrowDown') {
              event.preventDefault()
              browseHistory(-1)
            }
          }}
          disabled={state !== 'connected'}
          placeholder={state === 'connected' ? t('Typ een commando, bijvoorbeeld list') : state === 'connecting' ? t('Verbinden...') : t('Verbinding verbroken, opnieuw verbinden...')}
          className="flex-1 bg-transparent font-mono text-sm text-zinc-100 placeholder:text-zinc-500 focus:outline-none"
        />
        <span className={state === 'connected' ? 'size-2 rounded-full bg-green-500' : 'size-2 rounded-full bg-zinc-500'} title={state} />
      </form>
    </div>
  )
}
