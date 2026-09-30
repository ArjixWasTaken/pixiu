import { ref } from 'vue'

export type NowPlayingTab = 'queue' | 'lyrics' | 'about'

/** The expanded player: a panel over the main content on desktop, a sheet on phones. */
const open = ref(false)
const tab = ref<NowPlayingTab>('queue')
const about = ref<'Artist' | 'Album'>('Artist')

export const useNowPlaying = () => ({
  open,
  tab,
  about,
  toggle: () => (open.value = !open.value),
  close: () => (open.value = false),
  show: (on: NowPlayingTab = tab.value) => {
    tab.value = on
    open.value = true
  },
})
