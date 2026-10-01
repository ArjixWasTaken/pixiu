import { nextTick, reactive } from 'vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import * as lodash from 'lodash-es'
import { createHarness } from '@/__tests__/TestHarness'

vi.mock('lodash-es', async importOriginal => {
  const mod = await importOriginal<typeof lodash>()
  return { ...mod, shuffle: vi.fn(mod.shuffle) }
})
import { usePreferenceStore } from '@/stores/preferenceStore'
import { useQueueStore } from '@/stores/queueStore'
import { usePlayableStore } from '@/stores/playableStore'
import { useRecentlyPlayedStore } from '@/stores/recentlyPlayedStore'
import { logger } from '@/utils/logger'
import { playbackService } from '@/services/QueuePlaybackService'
import { useBranding } from '@/composables/useBranding'

describe('playbackService', () => {
  const h = createHarness({
    beforeEach: () => {
      usePlayableStore().vault.clear()
      h.createAudioPlayer()
      playbackService.activate(document.querySelector<HTMLMediaElement>('#audio-player')!)
    },
  })

  const setCurrentSong = (song?: Playable) => {
    const playbackState = song?.playback_state ?? 'Playing'
    const [synced] = usePlayableStore().syncWithVault(song || h.factory('song').make())
    synced.playback_state = playbackState
    useQueueStore().state.playables = reactive([synced])
    return synced
  }

  it('only initializes once', () => {
    const media = playbackService.media
    playbackService.activate(document.querySelector<HTMLMediaElement>('#audio-player')!)
    // media reference should remain the same (not re-initialized)
    expect(playbackService.media).toBe(media)
  })

  it.each([
    [false, 100, 400, 1],
    [true, 100, 400, 0],
    [false, 100, 500, 0],
  ])(
    'when playCountRegistered is %s, current media time is %d, media duration is %d, then registerPlay() should be call %d times',
    (playCountRegistered, currentTime, duration, numberOfCalls) => {
      const song = h.factory('song').make({
        play_count_registered: playCountRegistered,
        playback_state: 'Playing',
      })

      setCurrentSong(song)

      const mediaElement = playbackService.media

      // we can't set mediaElement.currentTime|duration directly because they're read-only
      h.setReadOnlyProperty(mediaElement, 'currentTime', currentTime)
      h.setReadOnlyProperty(mediaElement, 'duration', duration)

      const registerPlayMock = h.mock(playbackService, 'registerPlay')
      const saveMock = h.mock(useQueueStore(), 'savePlaybackStatus')

      mediaElement.dispatchEvent(new Event('timeupdate'))

      expect(registerPlayMock).toHaveBeenCalledTimes(numberOfCalls)
      expect(saveMock).toHaveBeenCalledWith(song, currentTime)
    },
  )

  it('plays next playable if current playable is errored', () => {
    const logMock = h.mock(logger, 'error')
    const playNextMock = h.mock(playbackService, 'playNext')

    const errorEvent = new Event('error')
    playbackService.media.dispatchEvent(errorEvent)

    expect(playNextMock).toHaveBeenCalled()
    expect(logMock).toHaveBeenCalledWith(errorEvent)
  })

  it.each<[RepeatMode, number, number]>([
    ['REPEAT_ONE', 1, 0],
    ['NO_REPEAT', 0, 1],
    ['REPEAT_ALL', 0, 1],
  ])(
    'when playable ends, if repeat mode is %s then restart() is called %d times and playNext() is called %d times',
    (repeatMode, restartCalls, playNextCalls) => {
      setCurrentSong()

      const restartMock = h.mock(playbackService, 'restart')
      const playNextMock = h.mock(playbackService, 'playNext')

      usePreferenceStore().repeat_mode = repeatMode

      playbackService.media.dispatchEvent(new Event('ended'))

      expect(restartMock).toHaveBeenCalledTimes(restartCalls)
      expect(playNextMock).toHaveBeenCalledTimes(playNextCalls)
    },
  )

  it.each([
    [true, 300, 310, 0],
    [false, 300, 400, 0],
    [false, 300, 310, 1],
  ])(
    'when next playable preloaded is %s, current media time is %d, media duration is %d, then preload() should be called %d times',
    (preloaded, currentTime, duration, numberOfCalls) => {
      setCurrentSong()
      h.mock(playbackService, 'registerPlay')
      h.setReadOnlyProperty(useQueueStore(), 'next', h.factory('song').make({ preloaded }))

      const mediaElement = playbackService.media

      h.setReadOnlyProperty(mediaElement, 'currentTime', currentTime)
      h.setReadOnlyProperty(mediaElement, 'duration', duration)

      const preloadMock = h.mock(playbackService, 'preload')
      h.mock(useQueueStore(), 'savePlaybackStatus')

      mediaElement.dispatchEvent(new Event('timeupdate'))

      expect(preloadMock).toHaveBeenCalledTimes(numberOfCalls)
    },
  )

  it('registers play', () => {
    const recentlyPlayedStoreAddMock = h.mock(useRecentlyPlayedStore(), 'add')
    const registerPlayMock = h.mock(usePlayableStore(), 'registerPlay')
    const song = h.factory('song').make()

    playbackService.registerPlay(song)

    expect(recentlyPlayedStoreAddMock).toHaveBeenCalledWith(song)
    expect(registerPlayMock).toHaveBeenCalledWith(song)
    expect(song.play_count_registered).toBe(true)
  })

  it('preloads a playable', () => {
    const audioElement = {
      setAttribute: vi.fn(),
      load: vi.fn(),
    }

    const createElementMock = h.mock(document, 'createElement', audioElement)
    h.mock(usePlayableStore(), 'getSourceUrl').mockReturnValue('/foo?token=o5afd')
    const song = h.factory('song').make()

    playbackService.preload(song)

    expect(createElementMock).toHaveBeenCalledWith('audio')
    expect(audioElement.setAttribute).toHaveBeenNthCalledWith(1, 'src', '/foo?token=o5afd')
    expect(audioElement.setAttribute).toHaveBeenNthCalledWith(2, 'preload', 'auto')
    expect(audioElement.load).toHaveBeenCalled()
    expect(song.preloaded).toBe(true)
  })

  it.each<[RepeatMode, RepeatMode]>([
    ['NO_REPEAT', 'REPEAT_ALL'],
    ['REPEAT_ALL', 'REPEAT_ONE'],
    ['REPEAT_ONE', 'NO_REPEAT'],
  ])('it switches from repeat mode %s to repeat mode %s', (fromMode, toMode) => {
    usePreferenceStore().repeat_mode = fromMode
    playbackService.rotateRepeatMode()

    expect(usePreferenceStore().repeat_mode).toEqual(toMode)
  })

  it('restarts playable if playPrev is triggered after 5 seconds', async () => {
    setCurrentSong()

    h.setReadOnlyProperty(playbackService.media, 'currentTime', 6)

    await playbackService.playPrev()

    expect(playbackService.media.currentTime).toBe(0)
  })

  it('stops if playPrev is triggered when there is no prev playable and repeat mode is NO_REPEAT', async () => {
    const stopMock = h.mock(playbackService, 'stop')
    h.setReadOnlyProperty(playbackService.media, 'currentTime', 4)
    h.setReadOnlyProperty(playbackService, 'previous', undefined)
    usePreferenceStore().repeat_mode = 'NO_REPEAT'

    await playbackService.playPrev()

    expect(stopMock).toHaveBeenCalled()
  })

  it('plays the previous playable', async () => {
    const previousSong = h.factory('song').make()
    h.setReadOnlyProperty(playbackService.media, 'currentTime', 4)
    h.setReadOnlyProperty(playbackService, 'previous', previousSong)
    const playMock = h.mock(playbackService, 'play')

    await playbackService.playPrev()

    expect(playMock).toHaveBeenCalledWith(previousSong)
  })

  it('stops if playNext is triggered when there is no next playable and repeat mode is NO_REPEAT', async () => {
    h.setReadOnlyProperty(playbackService, 'next', undefined)
    usePreferenceStore().repeat_mode = 'NO_REPEAT'
    const stopMock = h.mock(playbackService, 'stop')

    await playbackService.playNext()

    expect(stopMock).toHaveBeenCalled()
  })

  it('plays the next playable', async () => {
    const nextSong = h.factory('song').make()
    h.setReadOnlyProperty(playbackService, 'next', nextSong)
    const playMock = h.mock(playbackService, 'play')

    await playbackService.playNext()

    expect(playMock).toHaveBeenCalledWith(nextSong)
  })

  it('stops playback', () => {
    const currentSong = setCurrentSong()
    const pauseMock = h.mock(playbackService.media, 'pause')

    playbackService.stop()

    expect(currentSong.playback_state).toEqual('Stopped')
    expect(pauseMock).toHaveBeenCalled()
    expect(document.title).toEqual('Koel')
  })

  it('pauses playback', () => {
    const song = setCurrentSong()
    const pauseMock = h.mock(playbackService.media, 'pause')

    playbackService.pause()

    expect(song.playback_state).toEqual('Paused')
    expect(pauseMock).toHaveBeenCalled()
    expect(document.title).toEqual('Koel')
  })

  it('resumes playback', async () => {
    setCurrentSong(
      h.factory('song').make({
        title: 'Some song',
        playback_state: 'Paused',
      }),
    )

    const playMock = h.mock(window.HTMLMediaElement.prototype, 'play')

    await playbackService.resume()

    expect(useQueueStore().current?.playback_state).toEqual('Playing')
    expect(playMock).toHaveBeenCalled()
    expect(document.title).toEqual('Some song ♫ Koel')
  })

  it('plays first in queue if toggled when there is no current playable', async () => {
    useQueueStore().state.playables = []
    usePlayableStore().vault.clear()
    const playFirstInQueueMock = h.mock(playbackService, 'playFirstInQueue')

    await playbackService.toggle()

    expect(playFirstInQueueMock).toHaveBeenCalled()
  })

  it.each<[MethodOf<typeof playbackService>, PlaybackState]>([
    ['resume', 'Paused'],
    ['pause', 'Playing'],
  ])('%ss playback if toggled when current playable playback state is %s', async (action, playbackState) => {
    setCurrentSong(h.factory('song').make({ playback_state: playbackState }))
    const actionMock = h.mock(playbackService, action)
    await playbackService.toggle()

    expect(actionMock).toHaveBeenCalled()
  })

  it('queues and plays songs without shuffling', async () => {
    const songs = h.factory('song').make(5)
    const replaceQueueMock = h.mock(useQueueStore(), 'replaceQueueWith')
    const playMock = h.mock(playbackService, 'play')
    const firstSongInQueue = songs[0]
    h.setReadOnlyProperty(useQueueStore(), 'first', firstSongInQueue)

    playbackService.queueAndPlay(songs)
    await nextTick()

    expect(lodash.shuffle).not.toHaveBeenCalled()
    expect(replaceQueueMock).toHaveBeenCalledWith(songs)
    expect(playMock).toHaveBeenCalledWith(firstSongInQueue)
  })

  it('queues and plays songs with shuffling', async () => {
    const songs = h.factory('song').make(5)
    const shuffledSongs = h.factory('song').make(5)
    const replaceQueueMock = h.mock(useQueueStore(), 'replaceQueueWith')
    const playMock = h.mock(playbackService, 'play')
    const firstSongInQueue = songs[0]
    h.setReadOnlyProperty(useQueueStore(), 'first', firstSongInQueue)
    vi.mocked(lodash.shuffle).mockReturnValue(shuffledSongs)

    playbackService.queueAndPlay(songs, true)
    await nextTick()

    expect(lodash.shuffle).toHaveBeenCalledWith(songs)
    expect(replaceQueueMock).toHaveBeenCalledWith(shuffledSongs)
    expect(playMock).toHaveBeenCalledWith(firstSongInQueue)
  })

  it('plays first playable in queue', async () => {
    const songs = h.factory('song').make(5)
    useQueueStore().state.playables = songs
    h.setReadOnlyProperty(useQueueStore(), 'first', songs[0])
    const playMock = h.mock(playbackService, 'play')

    await playbackService.playFirstInQueue()

    expect(playMock).toHaveBeenCalledWith(songs[0])
  })

  it.each<[string, string | null]>([
    ['the playable cover', 'https://test/cover.jpg'],
    ['the branding cover', null],
  ])('sets the media session artwork to %s', (_, albumCover) => {
    const song = h.factory('song').make({ album_cover: albumCover! })
    usePreferenceStore().show_now_playing_notification = false

    playbackService.showNotification(song)

    const { artwork } = navigator.mediaSession.metadata!

    expect(artwork).toHaveLength(8)
    artwork!.forEach(image => expect(image.src).toBe(albumCover ?? useBranding().cover))
  })

  it('stops listening to media event after deactivation', () => {
    playbackService.deactivate()

    const logMock = h.mock(logger, 'error')
    const playNextMock = h.mock(playbackService, 'playNext')

    playbackService.media.dispatchEvent(new Event('error'))

    expect(playNextMock).not.toHaveBeenCalled()
    expect(logMock).not.toHaveBeenCalled()
  })
})
