import { screen } from '@testing-library/vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { commonStore } from '@/stores/commonStore'
import { preferenceStore as preferences } from '@/stores/preferenceStore'
import Component from './PreferencesForm.vue'

describe('preferencesForm', () => {
  const h = createHarness()

  it('has "Transcode on mobile" option if supported', () => {
    commonStore.state.supports_transcoding = true
    h.render(Component)
    screen.getByTestId('transcode_on_mobile')
  })

  it('does not have "Transcode on mobile" option if not supported', () => {
    commonStore.state.supports_transcoding = false
    h.render(Component)
    expect(screen.queryByTestId('transcode_on_mobile')).toBeNull()
  })

  it.each<[NotificationPermission, boolean]>([
    ['granted', true],
    ['denied', false],
  ])('asks for permission before showing notifications (%s)', async (answer, on) => {
    const requestPermission = vi.fn().mockResolvedValue(answer)
    vi.stubGlobal('Notification', { permission: 'default', requestPermission })
    preferences.show_now_playing_notification = false
    h.render(Component)

    await h.user.click(screen.getByRole('switch', { name: 'Show a notification when a song starts' }))

    expect(requestPermission).toHaveBeenCalled()
    expect(preferences.show_now_playing_notification).toBe(on)
    vi.unstubAllGlobals()
  })

  it('shows crossfade controls', () => {
    h.render(Component)
    screen.getByTestId('crossfade-slider')
    screen.getByTestId('crossfade-toggle')
  })

  it('toggles crossfade on via checkbox', async () => {
    preferences.crossfade_duration = 0
    h.render(Component)

    await h.user.click(screen.getByTestId('crossfade-toggle'))
    expect(preferences.crossfade_duration).toBe(7)
  })

  it('toggles crossfade off via checkbox', async () => {
    preferences.crossfade_duration = 5
    h.render(Component)

    await h.user.click(screen.getByTestId('crossfade-toggle'))
    expect(preferences.crossfade_duration).toBe(0)
  })
})
