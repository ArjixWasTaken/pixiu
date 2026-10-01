import { Hct, SchemeContent, hexFromArgb, sourceColorFromImageBytes } from '@material/material-color-utilities'

/** The Material 3 roles, as `DynamicScheme` names them; each is a `--schemes-*` variable. */
const ROLES = [
  'background',
  'onBackground',
  'surface',
  'surfaceDim',
  'surfaceBright',
  'surfaceContainerLowest',
  'surfaceContainerLow',
  'surfaceContainer',
  'surfaceContainerHigh',
  'surfaceContainerHighest',
  'onSurface',
  'surfaceVariant',
  'onSurfaceVariant',
  'inverseSurface',
  'inverseOnSurface',
  'outline',
  'outlineVariant',
  'shadow',
  'scrim',
  'surfaceTint',
  'primary',
  'onPrimary',
  'primaryContainer',
  'onPrimaryContainer',
  'inversePrimary',
  'secondary',
  'onSecondary',
  'secondaryContainer',
  'onSecondaryContainer',
  'tertiary',
  'onTertiary',
  'tertiaryContainer',
  'onTertiaryContainer',
  'error',
  'onError',
  'errorContainer',
  'onErrorContainer',
  'primaryFixed',
  'primaryFixedDim',
  'onPrimaryFixed',
  'onPrimaryFixedVariant',
  'secondaryFixed',
  'secondaryFixedDim',
  'onSecondaryFixed',
  'onSecondaryFixedVariant',
  'tertiaryFixed',
  'tertiaryFixedDim',
  'onTertiaryFixed',
  'onTertiaryFixedVariant',
] as const

type Role = (typeof ROLES)[number]

const variable = (role: Role) => `--schemes-${role.replace(/[A-Z]/g, letter => `-${letter.toLowerCase()}`)}`

/** Covers are read this small: plenty to find their colors. */
const SIZE = 112

const pixelsOf = (bitmap: ImageBitmap) => {
  const canvas =
    typeof OffscreenCanvas === 'function'
      ? new OffscreenCanvas(SIZE, SIZE)
      : Object.assign(document.createElement('canvas'), { width: SIZE, height: SIZE })

  const context = canvas.getContext('2d') as CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D | null

  if (!context) {
    return null
  }

  context.drawImage(bitmap, 0, 0, SIZE, SIZE)
  return context.getImageData(0, 0, SIZE, SIZE).data
}

/** Asks the server for a small copy, when it can make one. */
const smallCopy = (url: string) => {
  const parsed = new URL(url, window.location.href)

  if (parsed.pathname.endsWith('/getCoverArt') || parsed.pathname.endsWith('/getCoverArt.view')) {
    parsed.searchParams.set('size', String(SIZE))
  }

  return parsed.href
}

const readSourceColor = async (url: string) => {
  try {
    const response = await fetch(smallCopy(url))

    if (!response.ok) {
      return null
    }

    const bitmap = await createImageBitmap(await response.blob())
    const pixels = pixelsOf(bitmap)
    bitmap.close()

    return pixels ? sourceColorFromImageBytes(pixels) : null
  } catch {
    // Not an image, or not one this browser reads: the fixed colors do.
    return null
  }
}

const sources = new Map<string, Promise<number | null>>()

/** A cover's most characteristic color (ARGB), or null when it can't be read. Each cover is read once. */
export const sourceColorOf = (url: string) => {
  if (!sources.has(url)) {
    sources.set(url, readSourceColor(url))
  }

  return sources.get(url)!
}

/**
 * Every role, built around the source color the way M3's content schemes are:
 * they keep the cover's own hue, however muted.
 */
export const schemeFrom = (source: number, dark: boolean) => {
  const scheme = new SchemeContent(Hct.fromInt(source), dark, 0)

  return Object.fromEntries(ROLES.map(role => [variable(role), hexFromArgb(scheme[role])])) as Record<string, string>
}

const reducedMotion = () => window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false

let painted: string | null = null

/**
 * Sets the roles on the root, over the scheme's own (or takes them off with null).
 * The change fades, unless motion is reduced.
 */
export const paintRoot = (colors: Record<string, string> | null) => {
  const key = colors ? JSON.stringify(colors) : null

  if (key === painted) {
    return
  }

  painted = key

  const write = () => {
    const style = document.documentElement.style
    ROLES.forEach(role => style.removeProperty(variable(role)))
    colors && Object.entries(colors).forEach(([name, value]) => style.setProperty(name, value))
  }

  if (typeof document.startViewTransition === 'function' && !reducedMotion() && !document.hidden) {
    document.startViewTransition(write)
  } else {
    write()
  }
}
