import { screen, waitFor } from '@testing-library/vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import { defineComponent, ref } from 'vue'
import { CurrentStreamableKey } from '@/config/symbols'
import { createHarness } from '@/__tests__/TestHarness'
import { useCommonStore } from '@/stores/commonStore'
import { useQueueStore } from '@/stores/queueStore'
import { playbackService } from '@/services/QueuePlaybackService'
import Component from './QueueScreen.vue'

describe('queueScreen.vue', () => {
  const h = createHarness()

  const renderComponent = (playables: Playable[] = []) => {
    useQueueStore().state.playables = playables

    h.render(Component, {
      global: {
        stubs: {
          PlayableList: h.stub('song-list'),
        },
      },
    })
  }

  it('renders the queue', () => {
    renderComponent(h.factory('song').make(3))

    expect(screen.queryByTestId('song-list')).toBeTruthy()
    expect(screen.queryByTestId('screen-empty-state')).toBeNull()
  })

  it('renders an empty state if no songs queued', () => {
    renderComponent()

    expect(screen.queryByTestId('song-list')).toBeNull()
    expect(screen.queryByTestId('screen-empty-state')).toBeTruthy()
  })

  it('has an option to plays some random songs if the library is not empty', async () => {
    h.createAudioPlayer()

    useCommonStore().state.song_count = 300
    const fetchRandomMock = h.mock(useQueueStore(), 'fetchRandom')
    const playMock = h.mock(playbackService, 'playFirstInQueue')

    renderComponent()
    await h.user.click(screen.getByText('playing some random songs'))

    await waitFor(() => {
      expect(fetchRandomMock).toHaveBeenCalled()
      expect(playMock).toHaveBeenCalled()
    })
  })

  it('shuffles all', async () => {
    h.createAudioPlayer()

    const songs = h.factory('song').make(3)
    renderComponent(songs)
    const playMock = h.mock(playbackService, 'queueAndPlay')

    await h.user.click(screen.getByRole('button', { name: 'Shuffle' }))
    await waitFor(() => expect(playMock).toHaveBeenCalledWith(songs, true))
  })

  it('opens where it is playing, not at its top', async () => {
    const songs = h.factory('song').make(30)
    const scrollToPlayable = vi.fn()
    useQueueStore().state.playables = songs

    h.render(Component, {
      global: {
        provide: { [CurrentStreamableKey as symbol]: ref(songs[20]) },
        stubs: {
          PlayableList: defineComponent({
            setup: (_, { expose }) => expose({ scrollToPlayable }),
            template: '<div data-testid="song-list" />',
          }),
        },
      },
    })

    await waitFor(() => expect(scrollToPlayable).toHaveBeenCalledWith(songs[20]))
  })
})
