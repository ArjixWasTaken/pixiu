import { screen, waitFor } from '@testing-library/vue'
import { describe, expect, it } from 'vite-plus/test'
import { orderBy } from 'lodash-es'
import { createHarness } from '@/__tests__/TestHarness'
import { useQueueStore } from '@/stores/queueStore'
import { usePlayableStore } from '@/stores/playableStore'
import { playbackService } from '@/services/QueuePlaybackService'
import Component from './AlbumOrArtistThumbnail.vue'

describe('albumOrArtistThumbnail.vue', () => {
  const h = createHarness()

  const renderForAlbum = () => {
    const album = h.factory('album').make({
      name: 'IV',
      cover: 'https://test/album.jpg',
    })

    const rendered = h.render(Component, {
      props: {
        entity: album,
      },
    })

    return {
      ...rendered,
      album,
    }
  }

  const renderForArtist = () => {
    const artist = h.factory('artist').make({
      name: 'Led Zeppelin',
      image: 'https://test/blimp.jpg',
    })

    const rendered = h.render(Component, {
      props: {
        entity: artist,
      },
    })

    return {
      ...rendered,
      artist,
    }
  }

  it('renders for album', () => {
    expect(renderForAlbum().html()).toMatchSnapshot()
  })

  it('renders for artist', () => {
    expect(renderForArtist().html()).toMatchSnapshot()
  })

  it('plays album', async () => {
    h.createAudioPlayer()

    const songs = h.factory('song').make(10)
    const fetchMock = h.mock(usePlayableStore(), 'fetchSongsForAlbum').mockResolvedValue(songs)
    const playMock = h.mock(playbackService, 'queueAndPlay')
    const { album } = renderForAlbum()

    await h.user.click(screen.getByRole('button'))

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(album)
      expect(playMock).toHaveBeenCalledWith(songs)
    })
  })

  it('queues album', async () => {
    h.createAudioPlayer()

    const songs = h.factory('song').make(10)
    const fetchMock = h.mock(usePlayableStore(), 'fetchSongsForAlbum').mockResolvedValue(songs)
    const queueMock = h.mock(useQueueStore(), 'queue')
    const { album } = renderForAlbum()

    await h.user.keyboard('{Alt>}')
    await h.user.click(screen.getByRole('button'))
    await h.user.keyboard('{/Alt}')

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(album)
      expect(queueMock).toHaveBeenCalledWith(orderBy(songs, ['disc', 'track']))
    })
  })

  it('plays artist', async () => {
    const songs = h.factory('song').make(10)
    const fetchMock = h.mock(usePlayableStore(), 'fetchSongsForArtist').mockResolvedValue(songs)
    const playMock = h.mock(playbackService, 'queueAndPlay')
    const { artist } = renderForArtist()

    await h.user.click(screen.getByRole('button'))

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(artist)
      expect(playMock).toHaveBeenCalledWith(songs)
    })
  })

  it('queues artist', async () => {
    const songs = h.factory('song').make(10)
    const fetchMock = h.mock(usePlayableStore(), 'fetchSongsForArtist').mockResolvedValue(songs)
    const queueMock = h.mock(useQueueStore(), 'queue')
    const { artist } = renderForArtist()

    await h.user.keyboard('{Alt>}')
    await h.user.click(screen.getByRole('button'))
    await h.user.keyboard('{/Alt}')

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(artist)
      expect(queueMock).toHaveBeenCalledWith(orderBy(songs, ['album_id', 'disc', 'track']))
    })
  })
})
