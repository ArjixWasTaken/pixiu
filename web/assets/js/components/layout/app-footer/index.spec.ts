import { waitFor } from '@testing-library/vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { playbackService } from '@/services/QueuePlaybackService'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { playbackManager } from '@/services/playbackManager'
import Component from './index.vue'

describe('index.vue', () => {
  const h = createHarness()

  it('initializes playback and related services', async () => {
    h.createAudioPlayer()
    const useQueuePlaybackMock = h.mock(playbackManager, 'useQueuePlayback').mockReturnValue(playbackService)

    h.render(Component)
    usePreferenceStore().initialized = true

    await waitFor(() => expect(useQueuePlaybackMock).toHaveBeenCalled())
  })
})
