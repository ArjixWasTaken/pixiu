/**
 * The remote login browser: draws the server's screen onto a canvas and
 * sends mouse and keyboard input back over a WebSocket.
 *
 * Where the browser supports HTML-in-Canvas (an experimental API, see
 * https://github.com/WICG/html-in-canvas), the page's sign-in fields are
 * mirrored as real inputs drawn into the canvas, where password managers
 * can find and fill them. Other browsers get the same screen without the
 * mirrors.
 */

/** A sign-in field on the remote page, in page pixels. */
interface RemoteField {
  key: string
  kind: 'username' | 'password' | 'one_time_code'
  label: string
  value: string | null
  x: number
  y: number
  width: number
  height: number
}

/** The HTML-in-Canvas additions, not in TypeScript's DOM types yet. */
type DrawingCanvas = HTMLCanvasElement & { requestPaint?: () => void }
type DrawingContext = CanvasRenderingContext2D & {
  drawElementImage?: (element: Element, x: number, y: number, width: number, height: number) => DOMMatrix | void
}

interface Mirrors {
  update: (fields: RemoteField[]) => void
  layout: () => void
  stop: () => void
}

/** How each kind of field presents itself to password managers. */
const KINDS: Record<RemoteField['kind'], { type: string; autocomplete: AutoFill; name: string; inputMode?: string }> = {
  username: { type: 'text', autocomplete: 'username', name: 'username', inputMode: 'email' },
  password: { type: 'password', autocomplete: 'current-password', name: 'password' },
  one_time_code: { type: 'text', autocomplete: 'one-time-code', name: 'one-time-code', inputMode: 'numeric' },
}

/** Whether this browser can draw real inputs into the canvas. */
export const supportsMirrors = () => {
  const canvas = document.createElement('canvas') as DrawingCanvas
  const context = canvas.getContext('2d') as DrawingContext | null
  return typeof context?.drawElementImage === 'function' && typeof canvas.requestPaint === 'function'
}

/**
 * Shows the login browser on `canvas`, with sign-in fields mirrored into
 * `form` where supported. Returns a function that stops everything.
 */
export const startLoginScreen = (canvas: DrawingCanvas, form: HTMLFormElement, socketUrl: string) => {
  const context = canvas.getContext('2d') as DrawingContext
  let socket: WebSocket | null = null
  let stopped = false
  let reconnectTimer: number | undefined

  const send = (input: Record<string, unknown>) => {
    if (socket?.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify(input))
    }
  }

  // ---- the screen ----------------------------------------------------------

  let frame: ImageBitmap | null = null
  let received = 0
  let shown = 0
  let mirrors: Mirrors | null = null

  const draw = () => {
    if (mirrors) {
      canvas.requestPaint!()
    } else if (frame) {
      context.drawImage(frame, 0, 0, canvas.width, canvas.height)
    }
  }

  const receive = async (event: MessageEvent<string | Blob>) => {
    if (typeof event.data === 'string') {
      const message = JSON.parse(event.data)

      if (message.type === 'size') {
        // The page's size in CSS pixels; the canvas takes it on, so canvas
        // pixels are the coordinates input is replayed at.
        canvas.width = message.width
        canvas.height = message.height
        mirrors?.layout()
        draw()
      } else if (message.type === 'fields') {
        mirrors?.update(message.fields)
      }

      return
    }

    // Decoding is asynchronous; never let an older frame replace a newer one.
    const sequence = ++received
    const bitmap = await createImageBitmap(event.data)

    if (sequence < shown || stopped) {
      bitmap.close()
      return
    }

    shown = sequence
    frame?.close()
    frame = bitmap
    draw()
  }

  // A dropped connection (or a server restart) is picked up again; whoever
  // started the screen stops it once the login browser closes.
  const connect = () => {
    socket = new WebSocket(socketUrl)
    socket.binaryType = 'blob'
    socket.addEventListener('message', receive)
    socket.addEventListener('close', () => {
      if (!stopped) {
        reconnectTimer = window.setTimeout(connect, 1000)
      }
    })
  }

  // ---- input ---------------------------------------------------------------

  // Canvas pixels map one to one onto the remote page, whatever size the
  // canvas is shown at.
  const position = (event: MouseEvent) => {
    const rect = canvas.getBoundingClientRect()
    return {
      x: ((event.clientX - rect.left - canvas.clientLeft) * canvas.width) / canvas.clientWidth,
      y: ((event.clientY - rect.top - canvas.clientTop) * canvas.height) / canvas.clientHeight,
    }
  }

  // Events aimed at a mirrored field belong to it, not to the remote screen.
  const onScreen = (event: Event) => event.target === canvas

  // CDP's modifier bits.
  const modifiers = (event: KeyboardEvent) =>
    (event.altKey ? 1 : 0) | (event.ctrlKey ? 2 : 0) | (event.metaKey ? 4 : 0) | (event.shiftKey ? 8 : 0)

  let lastMove = 0
  const listeners: Array<[string, (event: any) => void, AddEventListenerOptions?]> = [
    [
      'mousemove',
      (event: MouseEvent) => {
        const now = performance.now()
        if (!onScreen(event) || now - lastMove < 40) return
        lastMove = now
        send({ type: 'mouse_move', ...position(event) })
      },
    ],
    [
      'mousedown',
      (event: MouseEvent) => {
        if (!onScreen(event)) return
        canvas.focus()
        send({ type: 'mouse_down', ...position(event), button: event.button })
        event.preventDefault()
      },
    ],
    [
      'mouseup',
      (event: MouseEvent) => {
        if (!onScreen(event)) return
        send({ type: 'mouse_up', ...position(event), button: event.button })
      },
    ],
    [
      'wheel',
      (event: WheelEvent) => {
        send({ type: 'wheel', ...position(event), dx: event.deltaX, dy: event.deltaY })
        event.preventDefault()
      },
      { passive: false },
    ],
    ['contextmenu', (event: MouseEvent) => onScreen(event) && event.preventDefault()],
    [
      'keydown',
      (event: KeyboardEvent) => {
        if (!onScreen(event)) return
        // Let the paste event deliver clipboard text.
        if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'v') return
        send({ type: 'key_down', key: event.key, code: event.code, modifiers: modifiers(event) })
        event.preventDefault()
      },
    ],
    [
      'keyup',
      (event: KeyboardEvent) => {
        if (!onScreen(event)) return
        send({ type: 'key_up', key: event.key, code: event.code, modifiers: modifiers(event) })
        event.preventDefault()
      },
    ],
    [
      'paste',
      (event: ClipboardEvent) => {
        if (!onScreen(event)) return
        const text = event.clipboardData?.getData('text')
        if (text) send({ type: 'text', text })
        event.preventDefault()
      },
    ],
  ]

  listeners.forEach(([name, listener, options]) => canvas.addEventListener(name, listener, options))

  // ---- mirrored sign-in fields (HTML-in-Canvas only) -------------------------

  // Each remote field gets a real input, drawn into the canvas over the
  // field's picture. The inputs are transparent, so what shows is always the
  // remote page; what is typed or filled into them is sent to the page.
  const createMirrors = (): Mirrors => {
    // Opts the canvas's children into layout and drawing, under the
    // attribute names of both revisions of the proposal. Only direct
    // children are hit tested, so a form elsewhere owns the inputs.
    canvas.setAttribute('layoutsubtree', '')
    canvas.setAttribute('content', 'drawable')

    interface Entry {
      input: HTMLInputElement
      field: RemoteField
      timer: number | undefined
    }

    const entries = new Map<string, Entry>()

    const fill = (entry: Entry) => {
      window.clearTimeout(entry.timer)
      entry.timer = undefined
      send({ type: 'fill', key: entry.field.key, value: entry.input.value })
    }

    // Typing bursts become one fill.
    const schedule = (entry: Entry) => {
      window.clearTimeout(entry.timer)
      entry.timer = window.setTimeout(() => fill(entry), 60)
    }

    // Enter, or a password manager submitting: send what is pending, then
    // press Enter in the page.
    const submit = (entry: Entry) => {
      for (const other of entries.values()) {
        if (other.timer) fill(other)
      }

      send({ type: 'focus', key: entry.field.key })
      send({ type: 'key_down', key: 'Enter', code: 'Enter', modifiers: 0 })
      send({ type: 'key_up', key: 'Enter', code: 'Enter', modifiers: 0 })
    }

    const create = (field: RemoteField) => {
      const kind = KINDS[field.kind] ?? KINDS.username
      const input = document.createElement('input')
      input.setAttribute('drawable', '')
      input.setAttribute('form', form.id)
      input.type = kind.type
      input.autocomplete = kind.autocomplete
      input.name = kind.name
      if (kind.inputMode) input.inputMode = kind.inputMode
      input.spellcheck = false
      input.autocapitalize = 'off'
      Object.assign(input.style, {
        display: 'block',
        boxSizing: 'border-box',
        margin: '0',
        padding: '0',
        border: '0',
        outline: 'none',
        background: 'transparent',
        color: 'transparent',
        webkitTextFillColor: 'transparent',
        caretColor: 'transparent',
        // Keeps the browser's own autofill tint from covering the field.
        transition: 'background-color 1000000s',
      })

      const entry: Entry = { input, field, timer: undefined }
      input.addEventListener('input', () => schedule(entry))
      input.addEventListener('change', () => fill(entry))
      input.addEventListener('focus', () => send({ type: 'focus', key: entry.field.key }))
      input.addEventListener('keydown', event => {
        if (event.key === 'Enter') {
          event.preventDefault()
          submit(entry)
        }
      })
      canvas.append(input)
      return entry
    }

    const onSubmit = (event: SubmitEvent) => {
      event.preventDefault()
      const all = [...entries.values()]
      const entry = all.find(({ input }) => input === document.activeElement) ?? all.at(-1)
      entry && submit(entry)
    }

    form.addEventListener('submit', onSubmit)

    // An input's CSS size must match its drawn size on screen, which depends
    // on how large the canvas is shown.
    const layout = () => {
      const scale = canvas.clientWidth / canvas.width

      for (const { input, field } of entries.values()) {
        input.style.width = `${field.width * scale}px`
        input.style.height = `${field.height * scale}px`
      }
    }

    const update = (fields: RemoteField[]) => {
      const keys = new Set(fields.map(field => field.key))

      for (const [key, entry] of entries) {
        if (!keys.has(key)) {
          window.clearTimeout(entry.timer)
          entry.input.remove()
          entries.delete(key)
        }
      }

      for (const field of fields) {
        let entry = entries.get(field.key)

        if (entry) {
          entry.field = field
        } else {
          entry = create(field)
          entries.set(field.key, entry)
        }

        entry.input.placeholder = field.label
        entry.input.setAttribute('aria-label', field.label)

        // Show what the page holds, unless the admin is busy in this field.
        if (field.value !== null && !entry.timer && document.activeElement !== entry.input) {
          entry.input.value = field.value
        }
      }

      layout()
      canvas.requestPaint!()
    }

    const paint = () => {
      context.reset()
      if (frame) context.drawImage(frame, 0, 0, canvas.width, canvas.height)

      for (const { input, field } of entries.values()) {
        try {
          const transform = context.drawElementImage!(input, field.x, field.y, field.width, field.height)

          // The first revision of the proposal leaves moving the element to
          // where it was drawn (for hit testing, and for password managers
          // measuring it) to the page; later ones do it themselves and
          // return nothing.
          if (transform) {
            const css = transform.toString()
            if (input.style.transform !== css) input.style.transform = css
          }
        } catch {
          // No snapshot of a new input yet; the next paint draws it.
        }
      }
    }

    canvas.addEventListener('paint', paint)

    const resizeObserver = new ResizeObserver(() => {
      layout()
      canvas.requestPaint!()
    })

    resizeObserver.observe(canvas)

    const stop = () => {
      resizeObserver.disconnect()
      canvas.removeEventListener('paint', paint)
      form.removeEventListener('submit', onSubmit)

      for (const entry of entries.values()) {
        window.clearTimeout(entry.timer)
        entry.input.remove()
      }

      entries.clear()
    }

    return { update, layout, stop }
  }

  if (typeof context.drawElementImage === 'function' && typeof canvas.requestPaint === 'function') {
    mirrors = createMirrors()
  }

  connect()
  canvas.focus()

  return () => {
    stopped = true
    window.clearTimeout(reconnectTimer)
    socket?.close()
    listeners.forEach(([name, listener, options]) => canvas.removeEventListener(name, listener, options))
    mirrors?.stop()
    frame?.close()
  }
}
