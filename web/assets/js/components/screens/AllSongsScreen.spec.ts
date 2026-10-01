import { screen, waitFor } from '@testing-library/vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import Router from '@/router'
import { useCommonStore } from '@/stores/commonStore'
import { useQueueStore } from '@/stores/queueStore'
import { usePlayableStore } from '@/stores/playableStore'
import { playbackService } from '@/services/QueuePlaybackService'
import Component from './AllSongsScreen.vue'

describe('allSongsScreen.vue', () => {
  const h = createHarness({
    beforeEach: () => {
      useCommonStore().state.song_count = 420
      useCommonStore().state.song_length = 123_456
      h.actingAsUser()
    },
  })

  const renderComponent = async () => {
    const fetchMock = h
      .mock(usePlayableStore(), 'paginateSongs')
      .mockResolvedValue({ items: h.factory('song').make(20), nextCursor: 'next-cursor-token' })

    await h.visit('/songs')

    const rendered = h.render(Component, {
      global: {
        stubs: {
          SongList: h.stub('song-list'),
        },
      },
    })

    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith({
        sort: 'title',
        order: 'asc',
        cursor: '',
      }),
    )

    return [rendered, fetchMock] as const
  }

  it('renders', async () => {
    const [{ html }] = await renderComponent()
    // Once its cover (loaded on demand) shows: the snapshot is of the screen as it settles.
    await waitFor(() => screen.getAllByTestId('thumbnail'))
    expect(html()).toMatchSnapshot()
  })

  it('shuffles', async () => {
    h.createAudioPlayer()

    const queueMock = h.mock(useQueueStore(), 'fetchRandom')
    const playMock = h.mock(playbackService, 'playFirstInQueue')
    const goMock = h.mock(Router, 'go')
    await renderComponent()

    await h.user.click(await screen.findByRole('button', { name: 'Shuffle' }))

    await waitFor(() => {
      expect(queueMock).toHaveBeenCalled()
      expect(playMock).toHaveBeenCalled()
      expect(goMock).toHaveBeenCalledWith('/queue')
    })
  })
})
