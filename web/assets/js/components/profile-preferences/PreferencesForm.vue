<template>
  <div class="space-y-4">
    <label class="pref-row">
      <span>Playing a song plays the rest of its playlist, album, artist or genre</span>
      <M3Switch v-model="preferences.continuous_playback" name="continuous_playback" />
    </label>
    <label v-if="supportsNotifications" class="pref-row">
      <span>Show a notification when a song starts</span>
      <M3Switch :model-value="notifying" name="notify" @update:model-value="toggleNotifications" />
    </label>
    <label v-if="!onMobile" class="pref-row">
      <span>Confirm before closing {{ appName }}</span>
      <M3Switch v-model="preferences.confirm_before_closing" name="confirm_closing" />
    </label>
    <div v-if="showTranscodingOption" class="pref-row">
      <span>
        Convert and play media at
        <select
          v-model="preferences.transcode_quality"
          :disabled="!preferences.transcode_on_mobile"
          class="appearance-auto rounded-sm"
        >
          <option v-for="quality in [64, 96, 128, 192, 256, 320]" :key="quality" :value="quality">
            {{ quality }}
          </option>
        </select>
        kbps on mobile
      </span>
      <M3Switch
        v-model="preferences.transcode_on_mobile"
        data-testid="transcode_on_mobile"
        name="transcode_on_mobile"
      />
    </div>
    <div class="pref-row">
      <span class="flex-1">
        <span class="flex items-center gap-3">
          <label id="crossfade-label" for="crossfade-slider" class="shrink-0">Crossfade songs</label>
          <input
            id="crossfade-slider"
            v-model.number="preferences.crossfade_duration"
            type="range"
            min="0"
            max="15"
            step="1"
            data-testid="crossfade-slider"
            class="crossfade-slider flex-1 min-w-32 max-w-96"
          />
          <span class="text-(--schemes-on-surface-variant) shrink-0">
            {{ crossfadeEnabled ? `${preferences.crossfade_duration}s` : 'Off' }}
          </span>
        </span>
      </span>
      <M3Switch
        :model-value="crossfadeEnabled"
        name="crossfade"
        data-testid="crossfade-toggle"
        @update:model-value="toggleCrossfade"
      />
    </div>
  </div>
</template>

<script lang="ts" setup>
import { useViewport } from '@/composables/useViewport'
import { computed, ref, toRef } from 'vue'
import { useCommonStore } from '@/stores/commonStore'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { useBranding } from '@/composables/useBranding'

import M3Switch from '@/components/m3/M3Switch.vue'

const commonStore = useCommonStore()
const preferences = usePreferenceStore()

const { isTouch: onMobile } = useViewport()

const { name: appName } = useBranding()

const showTranscodingOption = toRef(commonStore.state, 'supports_transcoding')

const supportsNotifications = 'Notification' in window
const permission = ref(supportsNotifications ? Notification.permission : 'denied')
const notifying = computed(() => preferences.show_now_playing_notification && permission.value === 'granted')

/** Asks for the browser's permission when turned on, and stays off without it. */
const toggleNotifications = async (enabled: boolean | undefined) => {
  if (enabled && permission.value !== 'granted') {
    permission.value = await Notification.requestPermission()
  }

  preferences.show_now_playing_notification = Boolean(enabled) && permission.value === 'granted'
}

const crossfadeEnabled = computed(() => preferences.crossfade_duration > 0)

const toggleCrossfade = (enabled: boolean | undefined) => {
  preferences.crossfade_duration = enabled ? 7 : 0
}
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
.pref-row {
  @apply flex items-center gap-4 cursor-pointer;

  > :first-child {
    @apply flex-1;
  }
}

.crossfade-slider {
  appearance: none;
  height: 4px;
  border-radius: 2px;
  outline: none;
  @apply bg-(--schemes-surface-container-highest);
  cursor: pointer;
}

.crossfade-slider::-webkit-slider-thumb {
  appearance: none;
  height: 14px;
  width: 14px;
  border-radius: 50%;
  border: 0;
  cursor: pointer;
  @apply bg-(--schemes-on-surface);
}

.crossfade-slider::-moz-range-thumb {
  height: 14px;
  width: 14px;
  border-radius: 50%;
  border: 0;
  cursor: pointer;
  @apply bg-(--schemes-on-surface);
}
</style>
