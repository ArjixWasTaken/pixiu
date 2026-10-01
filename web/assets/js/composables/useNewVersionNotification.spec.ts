import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'

const mockManageSettings = vi.fn()

vi.mock('@/composables/usePolicies', () => ({
  usePolicies: () => ({
    currentUserCan: {
      manageSettings: mockManageSettings,
    },
  }),
}))

import { useCommonStore } from '@/stores/commonStore'
import { useNewVersionNotification } from './useNewVersionNotification'

describe('useNewVersionNotification', () => {
  createHarness({
    beforeEach: () => {
      useCommonStore().state.latest_version = '7.0.0'
      useCommonStore().state.current_version = '6.5.0'
      mockManageSettings.mockReturnValue(true)
    },
  })

  it('detects new version available', () => {
    const { shouldNotifyNewVersion } = useNewVersionNotification()
    expect(shouldNotifyNewVersion.value).toBe(true)
  })

  it('does not notify when versions are equal', () => {
    useCommonStore().state.latest_version = '6.5.0'
    const { shouldNotifyNewVersion } = useNewVersionNotification()
    expect(shouldNotifyNewVersion.value).toBe(false)
  })

  it('does not notify when current is newer', () => {
    useCommonStore().state.current_version = '8.0.0'
    const { shouldNotifyNewVersion } = useNewVersionNotification()
    expect(shouldNotifyNewVersion.value).toBe(false)
  })

  it('does not notify when user cannot manage settings', () => {
    mockManageSettings.mockReturnValue(false)
    const { shouldNotifyNewVersion } = useNewVersionNotification()
    expect(shouldNotifyNewVersion.value).toBe(false)
  })

  it('exposes version refs', () => {
    const { currentVersion, latestVersion } = useNewVersionNotification()
    expect(currentVersion.value).toBe('6.5.0')
    expect(latestVersion.value).toBe('7.0.0')
  })

  it('handles leading v prefix on either side', () => {
    useCommonStore().state.latest_version = 'v9.2.1'
    useCommonStore().state.current_version = 'v9.2.0'
    const { shouldNotifyNewVersion } = useNewVersionNotification()
    expect(shouldNotifyNewVersion.value).toBe(true)
  })

  it('compares double-digit segments numerically, not lexicographically', () => {
    useCommonStore().state.latest_version = '9.10.0'
    useCommonStore().state.current_version = '9.9.0'
    const { shouldNotifyNewVersion } = useNewVersionNotification()
    expect(shouldNotifyNewVersion.value).toBe(true)
  })

  it('treats pre-release suffix as not newer than the base release', () => {
    useCommonStore().state.latest_version = '9.2.0-beta.1'
    useCommonStore().state.current_version = '9.2.0'
    const { shouldNotifyNewVersion } = useNewVersionNotification()
    expect(shouldNotifyNewVersion.value).toBe(false)
  })
})
