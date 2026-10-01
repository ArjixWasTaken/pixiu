import { merge } from 'lodash-es'
import { reactive } from 'vue'

import PlayableListControls from '@/components/playable/playable-list/PlayableListControls.vue'

export const usePlayableListControls = (
  screen: ScreenName,
  configOverrides: Partial<PlayableListControlsConfig> | (() => Partial<PlayableListControlsConfig>) = {},
) => {
  const defaults: PlayableListControlsConfig = {
    addTo: {
      queue: screen !== 'Queue',
      favorites: screen !== 'Favorites',
    },
    clearQueue: screen === 'Queue',
    refresh: screen === 'Playlist',
  }

  const config = merge(defaults, typeof configOverrides === 'function' ? configOverrides() : configOverrides)

  return {
    PlayableListControls,
    config: reactive(config),
  }
}
