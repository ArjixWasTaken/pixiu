import { equalizerStore } from '@/stores/equalizerStore'
import { preferenceStore } from '@/stores/preferenceStore'
import { frequencies } from '@/config/audio'

export const dbToGain = (db: number) => 10 ** (db / 20) || 0

export interface Band {
  label: string
  node: BiquadFilterNode
  db: number
}

export const audioService = {
  unlocked: false,

  context: null! as AudioContext,
  source: null! as MediaElementAudioSourceNode,
  element: null! as HTMLMediaElement,
  preampGainNode: null! as GainNode,
  analyzer: null! as AnalyserNode,

  bands: [] as Band[],
  preamp: 0,
  /** Whether audio plays past the equalizer. */
  bypassed: false,

  /** Where the source plays into: the equalizer, or straight past it. */
  entry(): AudioNode {
    return this.bypassed ? this.analyzer : this.preampGainNode
  },

  /** Switches the equalizer off (`true`) or back on, keeping its settings. */
  setBypassed(bypassed: boolean) {
    if (bypassed === this.bypassed) {
      return
    }
    this.bypassed = bypassed

    if (!this.source) {
      return
    }
    try {
      this.source.disconnect()
    } catch {
      // may already be disconnected
    }
    this.source.connect(this.entry())
  },

  init(mediaElement: HTMLMediaElement) {
    this.element = mediaElement

    this.context = new AudioContext()
    this.preampGainNode = this.context.createGain()
    this.source = this.context.createMediaElementSource(this.element)
    this.analyzer = this.context.createAnalyser()
    this.bypassed = !preferenceStore.equalizer_enabled

    this.source.connect(this.entry())

    const config = equalizerStore.getConfig()

    this.changePreampGain(config.preamp)

    let prevFilter: BiquadFilterNode

    // Create 10 bands with the frequencies similar to those of Winamp and connect them together.
    frequencies.forEach((frequency, i) => {
      const filter = this.context.createBiquadFilter()

      if (i === 0) {
        filter.type = 'lowshelf'
      } else if (i === frequencies.length - 1) {
        filter.type = 'highshelf'
      } else {
        filter.type = 'peaking'
      }

      filter.Q.setTargetAtTime(1, this.context.currentTime, 0.01)
      filter.frequency.setTargetAtTime(frequency, this.context.currentTime, 0.01)
      filter.gain.value = dbToGain(config.gains[i])

      prevFilter ? prevFilter.connect(filter) : this.preampGainNode.connect(filter)
      prevFilter = filter

      this.bands.push({
        node: filter,
        label: String(frequency).replace('000', 'K'),
        db: config.gains[i],
      })
    })

    prevFilter!.connect(this.analyzer)

    // connect the analyzer node last, so that changes to the equalizer affect the visualizer as well
    this.analyzer.connect(this.context.destination)

    this.unlockAudioContext()
  },

  reconnectSource(newElement: HTMLMediaElement) {
    try {
      this.source.disconnect()
    } catch {
      // may already be disconnected
    }

    this.element = newElement
    this.source = this.context.createMediaElementSource(newElement)
    this.source.connect(this.entry())
  },

  changePreampGain(db: number) {
    this.preamp = db
    this.preampGainNode.gain.value = dbToGain(db)
  },

  changeFilterGain(node: BiquadFilterNode, db: number) {
    node.gain.value = dbToGain(db)
  },

  /**
   * Attempt to unlock the audio context on mobile devices by creating and playing a silent buffer upon the
   * first user interaction.
   */
  unlockAudioContext() {
    ;['touchend', 'touchstart', 'click'].forEach(event => {
      document.addEventListener(
        event,
        () => {
          if (this.unlocked) {
            return
          }

          const source = this.context.createBufferSource()
          source.buffer = this.context.createBuffer(1, 1, 22050)
          source.connect(this.context.destination)
          source.start(0)

          this.unlocked = true
        },
        {
          once: true,
        },
      )
    })
  },
}
