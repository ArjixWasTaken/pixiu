import { computed, useAttrs } from 'vue'

/**
 * For fields that wrap an input: `class` and `style` style the wrapper,
 * every other attribute and listener goes to the input.
 */
export const useSplitAttrs = () => {
  const attrs = useAttrs()

  const rootAttrs = computed(() => ({ class: attrs.class, style: attrs.style as any }))

  const inputAttrs = computed(() => {
    const { class: _class, style: _style, ...rest } = attrs
    return rest
  })

  return { rootAttrs, inputAttrs }
}
