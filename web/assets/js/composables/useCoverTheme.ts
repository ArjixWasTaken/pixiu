import type { Ref } from 'vue'
import { ref, watch } from 'vue'
import { paintRoot, schemeFrom, sourceColorOf } from '@/services/coverColors'
import { useQueueStore } from '@/stores/queueStore'
import { useThemeStore } from '@/stores/themeStore'
/**
 * With "From what's playing" chosen, the whole player takes its colors from
 * the cover playing; with nothing playing, or a cover it can't read, Orange's.
 * Call once, from the app's shell.
 */
export const useCoverTheme = () => {
  let latest = 0

  watch(
    [() => useQueueStore().current?.album_cover, () => useThemeStore().followsCover, () => useThemeStore().state.dark],
    async ([cover, follows, dark]) => {
      const run = ++latest
      const source = follows && cover ? await sourceColorOf(cover) : null

      // A later song (or setting) has taken over meanwhile.
      if (run !== latest) {
        return
      }

      paintRoot(source === null ? null : schemeFrom(source, dark))
    },
    { immediate: true },
  )
}

/** A cover's own primary color, in the mode showing, for tinting what it heads. */
export const useCoverTint = (cover: Ref<string | null | undefined>) => {
  const tint = ref<string | null>(null)
  let latest = 0

  watch(
    [cover, () => useThemeStore().state.dark],
    async ([url, dark]) => {
      const run = ++latest
      const source = url ? await sourceColorOf(url) : null

      if (run === latest) {
        tint.value = source === null ? null : schemeFrom(source, dark)['--schemes-primary']
      }
    },
    { immediate: true },
  )

  return tint
}
