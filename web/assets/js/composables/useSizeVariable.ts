import { onScopeDispose, ref } from 'vue'

/** Where m3.pcss switches to the compact sizes. */
export const COMPACT_QUERY = '(pointer: fine) and (min-width: 769px)'

/**
 * A size variable from m3.pcss (`--m3-row-height`), in px, for code that lays
 * things out itself. It follows the density as the window or pointer changes.
 */
export const useSizeVariable = (name: `--${string}`, fallback: number) => {
  const read = () => parseFloat(getComputedStyle(document.documentElement).getPropertyValue(name)) || fallback

  const value = ref(read())
  const query = window.matchMedia?.(COMPACT_QUERY)
  const onChange = () => (value.value = read())

  query?.addEventListener?.('change', onChange)
  onScopeDispose(() => query?.removeEventListener?.('change', onChange))

  return value
}
