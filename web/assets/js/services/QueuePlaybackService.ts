import type { Ref } from 'vue'
import { ref } from 'vue'
import { shuffle } from 'lodash-es'
import { useCommonStore } from '@/stores/commonStore'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { useQueueStore } from '@/stores/queueStore'
import { useRecentlyPlayedStore } from '@/stores/recentlyPlayedStore'
import { usePlayableStore } from '@/stores/playableStore'
import { logger } from '@/utils/logger'
import { arrayify } from '@/utils/helpers'
import { isAudioContextSupported } from '@/utils/supports'
import { audioService } from '@/services/audioService'
import { subsonic } from '@/services/subsonic'
import { BasePlaybackService } from '@/services/BasePlaybackService'
import { crossfadeService } from '@/services/crossfadeService'
import { encyclopediaService } from '@/services/encyclopediaService'
import { volumeManager } from '@/services/volumeManager'
import { useBranding } from '@/composables/useBranding'

/**
 * The number of seconds before the current playable ends to start preloading the next one.
 */
const PRELOAD_BUFFER = 30

export class QueuePlaybackService extends BasePlaybackService {
  private repeatModes: RepeatMode[] = ['NO_REPEAT', 'REPEAT_ALL', 'REPEAT_ONE']
  private upNext: Ref<Playable | null> = ref(null)

  /**
   * The next item in the queue.
   * If we're in REPEAT_ALL mode and there's no next item, just get the first item.
   */
  public get next() {
    if (useQueueStore().next) {
      return useQueueStore().next
    }

    return usePreferenceStore().repeat_mode === 'REPEAT_ALL' ? useQueueStore().first : undefined
  }

  /**
   * The previous item in the queue.
   * If we're in REPEAT_ALL mode and there's no prev item, get the last item.
   */
  public get previous() {
    if (useQueueStore().previous) {
      return useQueueStore().previous
    }

    return usePreferenceStore().repeat_mode === 'REPEAT_ALL' ? useQueueStore().last : undefined
  }

  public registerPlay(playable: Playable) {
    useRecentlyPlayedStore().add(playable)
    usePlayableStore().registerPlay(playable)
    playable.play_count_registered = true

    if (!playable.album_cover) {
      encyclopediaService.fetchForAlbum({ id: playable.album_id } as Album).catch(logger.error)
    }
  }

  public preload(playable: Playable) {
    const audioElement = document.createElement('audio')
    audioElement.setAttribute('src', usePlayableStore().getSourceUrl(playable))
    audioElement.setAttribute('preload', 'auto')
    audioElement.load()
    playable.preloaded = true
  }

  /**
   * Play a song. Because
   *
   * So many adventures couldn't happen today,
   * So many songs we forgot to play
   * So many dreams swinging out of the blue
   * We'll let them come true
   */
  public async play(playable: Playable, position = 0) {
    const isCrossfadeFinalization = crossfadeService.active && crossfadeService.state!.playable.id === playable.id

    // Cancel any active crossfade unless we're finalizing it
    if (!isCrossfadeFinalization) {
      this.cancelCrossfade()
    }

    useQueueStore().queueIfNotQueued(playable, 'after-current')

    // If for any reason (most likely a bug), the requested playable has been deleted, attempt the next item in the queue.
    if (playable.deleted) {
      logger.warn('Attempted to play a deleted playable', playable)

      if (this.next && this.next.id !== playable.id) {
        await this.playNext()
      }

      return
    }

    const current = useQueueStore().current

    if (current) {
      current.playback_state = 'Stopped'
    }

    playable.playback_state = 'Playing'

    await this.setNowPlayingMeta(playable)

    if (isCrossfadeFinalization) {
      // The incoming track is already playing via the crossfade audio element.
      // Simply swap it in as the new primary — no src change, no seeking, no interruption.
      const { incomingAudio } = crossfadeService.state!

      // Stop and fully discard the old element
      this.media.pause()
      this.media.removeAttribute('src')
      this.media.load()

      // The incoming audio is already playing at the right position.
      // Just make it the new primary media element.
      this.swapMediaElement(incomingAudio)
      this.setVolume(volumeManager.get())

      // Reconnect the audio graph to the new element
      if (isAudioContextSupported && audioService.context) {
        audioService.reconnectSource(incomingAudio)
      }

      crossfadeService.state = null

      this.recordStartTime(playable)
      this.showNotification(playable)
    } else {
      // Normal playback: set src and start
      this.media.src = usePlayableStore().getSourceUrl(playable)

      if (position === 0) {
        await this.restart()
      } else {
        this.seekTo(position)
        await this.resume()
      }
    }

    this.setMediaSessionActionHandlers()
  }

  public showNotification(playable: Playable) {
    if (usePreferenceStore().show_now_playing_notification && window.Notification?.permission === 'granted') {
      try {
        const notification = new window.Notification(`♫ ${playable.title}`, {
          icon: playable.album_cover,
          body: `${playable.album_name} – ${playable.artist_name}`,
        })

        notification.onclick = () => window.focus()

        window.setTimeout(() => notification.close(), 5000)
      } catch (error: unknown) {
        // Notification fails.
        // @link https://developer.mozilla.org/en-US/docs/Web/API/ServiceWorkerRegistration/showNotification
        logger.error(error)
      }
    }

    if (!navigator.mediaSession) {
      return
    }

    const cover = playable.album_cover || useBranding().cover

    navigator.mediaSession.metadata = new MediaMetadata({
      title: playable.title,
      artist: playable.artist_name,
      album: playable.album_name,
      artwork: [48, 64, 96, 128, 192, 256, 384, 512].map(d => ({
        src: cover,
        sizes: `${d}x${d}`,
      })),
    })
  }

  public async restart() {
    const playable = useQueueStore().current!

    // Reset the "up next" value to let subscribers know that the next item is cleared
    // (because another playable, likely the "next" one, is being played)
    this.upNext.value = null

    this.recordStartTime(playable)

    useQueueStore().savePlaybackStatus(playable, 0)
    subsonic.scrobble(playable.id, false).catch(error => logger.error(error))

    this.media.currentTime = 0

    try {
      await this.media.play()
      navigator.mediaSession && (navigator.mediaSession.playbackState = 'playing')
      this.showNotification(playable)
    } catch (error: unknown) {
      // convert this into a warning to avoid breaking the app
      logger.warn(error)
    }
  }

  public rotateRepeatMode() {
    let index = this.repeatModes.indexOf(usePreferenceStore().repeat_mode) + 1

    if (index >= this.repeatModes.length) {
      index = 0
    }

    usePreferenceStore().repeat_mode = this.repeatModes[index]
  }

  /**
   * Play the prev item the queue, if one is found.
   * If there's no prev item and the current mode is NO_REPEAT, we stop completely.
   */
  public async playPrev() {
    // If the item's duration is greater than 5 seconds, and we've passed 5 seconds into it,
    // restart playing instead.
    if (this.media.currentTime > 5 && useQueueStore().current!.length > 5) {
      this.media.currentTime = 0

      return
    }

    if (!this.previous && usePreferenceStore().repeat_mode === 'NO_REPEAT') {
      await this.stop()
    } else {
      this.previous && (await this.play(this.previous))
    }
  }

  /**
   * Play the next item in the queue if one is found.
   * If there's no next item and the current mode is NO_REPEAT, we stop completely.
   */
  public async playNext() {
    if (!this.next && usePreferenceStore().repeat_mode === 'NO_REPEAT') {
      await this.stop() //  Nothing lasts forever, even cold November rain.
    } else {
      this.next && (await this.play(this.next))
    }
  }

  public async stop() {
    this.cancelCrossfade()

    if (this.media) {
      this.media.pause()
      this.seekTo(0)
    }

    document.title = useBranding().name

    const current = useQueueStore().current
    current && (current.playback_state = 'Stopped')

    navigator.mediaSession && (navigator.mediaSession.playbackState = 'none')
  }

  public async pause() {
    this.cancelCrossfade()
    this.media.pause()

    useQueueStore().current!.playback_state = 'Paused'
    navigator.mediaSession && (navigator.mediaSession.playbackState = 'paused')
    // The tab names the song only while it plays.
    document.title = useBranding().name
  }

  public async resume() {
    const playable = useQueueStore().current!

    if (!this.media.src) {
      // on first load when the queue is loaded from saved state, the player's src is empty
      // we need to properly set it as well as any kind of playback metadata
      this.media.src = usePlayableStore().getSourceUrl(playable)
      this.seekTo(useCommonStore().state.queue_state.playback_position)

      await this.setNowPlayingMeta(useQueueStore().current!)
      this.recordStartTime(playable)
    }

    try {
      await this.media.play()
    } catch (error: unknown) {
      logger.error(error)
    }

    useQueueStore().current!.playback_state = 'Playing'
    navigator.mediaSession && (navigator.mediaSession.playbackState = 'playing')
    document.title = `${playable.title} ♫ ${useBranding().name}`
  }

  public async toggle() {
    if (!useQueueStore().current) {
      await this.playFirstInQueue()
      return
    }

    if (useQueueStore().current?.playback_state !== 'Playing') {
      await this.resume()
      return
    }

    this.pause()
  }

  /**
   * Queue up playables (replace them into the queue) and start playing right away.
   */
  public async queueAndPlay(playables: MaybeArray<Playable>, shuffled = false) {
    playables = arrayify(playables)

    if (shuffled) {
      playables = shuffle(playables)
    }

    await this.stop()
    useQueueStore().replaceQueueWith(playables)
    await this.play(useQueueStore().first)
  }

  public async playFirstInQueue() {
    useQueueStore().all.length && (await this.play(useQueueStore().first))
  }

  private async setNowPlayingMeta(playable: Playable) {
    document.title = `${playable.title} ♫ ${useBranding().name}`
    this.media.setAttribute('title', `${playable.artist_name} - ${playable.title}`)

    if (isAudioContextSupported) {
      await audioService.context.resume()
    }
  }

  // Record the UNIX timestamp the playable starts playing, for scrobbling purpose
  private recordStartTime(song: Playable) {
    song.play_start_time = Math.floor(Date.now() / 1000)
    song.play_count_registered = false
  }

  public forward(seconds: number): void {
    this.media.currentTime += seconds
  }

  protected onEnded(): void {
    // If a crossfade is active (completed or not), the outgoing track has ended — transition to the next song
    if (crossfadeService.active) {
      const { playable } = crossfadeService.state!
      this.play(playable)
      return
    }

    usePreferenceStore().repeat_mode === 'REPEAT_ONE' ? this.restart() : this.playNext()
  }

  protected onError(error: ErrorEvent): void {
    logger.error(error)
    this.playNext()
  }

  protected onTimeUpdate(): void {
    const currentPlayable = useQueueStore().current

    if (!currentPlayable) {
      return
    }

    const media = this.media

    // If we've passed 25% of the playable, it's safe to say it has been "played".
    // See https://github.com/koel/koel/issues/1087
    if (!currentPlayable.play_count_registered && media.currentTime * 4 >= media.duration) {
      this.registerPlay(currentPlayable)
    }

    if (Math.ceil(media.currentTime) % 5 === 0) {
      // every 5 seconds, we save the current playback position to the server
      useQueueStore().savePlaybackStatus(currentPlayable, Math.ceil(media.currentTime))
    }

    const nextPlayable = useQueueStore().next

    if (!nextPlayable) {
      return
    }

    // Set the "up next" value to the next playable if we're near the end of the current playback.
    this.upNext.value = media.currentTime + 15 > media.duration ? nextPlayable : null

    // Preload the next playable if we're near the end of the current playback.
    if (media.currentTime + PRELOAD_BUFFER > media.duration && !nextPlayable.preloaded) {
      this.preload(nextPlayable)
    }

    // Initiate crossfade if enabled and near the end of the track
    const crossfadeDuration = usePreferenceStore().crossfade_duration

    if (
      crossfadeDuration > 0 &&
      !crossfadeService.active &&
      usePreferenceStore().repeat_mode !== 'REPEAT_ONE' &&
      media.duration > crossfadeDuration * 2 && // skip for short tracks
      media.currentTime + crossfadeDuration >= media.duration
    ) {
      if (crossfadeService.start(nextPlayable, crossfadeDuration, volumeManager.get())) {
        // Show the incoming track as "now playing" immediately
        useQueueStore().current!.playback_state = 'Stopped'
        nextPlayable.playback_state = 'Playing'
        this.setNowPlayingMeta(nextPlayable)
        this.showNotification(nextPlayable)
        this.registerPlay(nextPlayable)
      }
    }

    // Fade out the primary player during an active crossfade
    if (crossfadeService.active && crossfadeService.state) {
      const remaining = media.duration - media.currentTime
      const progress = Math.max(0, 1 - remaining / crossfadeDuration)
      this.setVolume(volumeManager.get() * (1 - progress))
    }
  }

  public rewind(seconds: number): void {
    this.media.currentTime -= seconds
  }

  public fastSeek(position: number): void {
    this.media.fastSeek(position || 0)
  }

  public seekTo(position: number): void {
    this.cancelCrossfade()
    this.media.currentTime = position || 0
  }

  /** Cancel any active crossfade and restore volume */
  private cancelCrossfade() {
    if (crossfadeService.active) {
      crossfadeService.cancel()
      this.setVolume(volumeManager.get())
    }
  }
}

export const playbackService = new QueuePlaybackService()
