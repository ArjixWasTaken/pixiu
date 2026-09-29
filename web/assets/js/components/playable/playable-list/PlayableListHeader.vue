<template>
  <div v-if="config.sortable" ref="container" class="sort-bar">
    <M3Chip
      :icon="sortOrder === 'asc' ? 'arrow_upward' : 'arrow_downward'"
      :title="`Sorted by ${currentLabel}, ${sortOrder === 'asc' ? 'ascending' : 'descending'}`"
      data-testid="sort-chip"
      @click="open = !open"
    >
      {{ currentLabel }}
    </M3Chip>

    <M3Menu v-if="open" class="menu" data-testid="sort-menu">
      <M3MenuItem
        v-for="option in options"
        :key="option.label"
        :selected="isCurrent(option.field)"
        :label="option.label"
        tag="div"
        @click="sort(option.field)"
      >
        <template #trailing>
          <M3Icon
            v-if="isCurrent(option.field)"
            :name="sortOrder === 'asc' ? 'arrow_upward' : 'arrow_downward'"
            :size="18"
          />
        </template>
      </M3MenuItem>
    </M3Menu>
  </div>
</template>

<script setup lang="ts">
import type { Ref } from 'vue'
import { computed, ref, useTemplateRef } from 'vue'
import { onClickOutside } from '@vueuse/core'
import { arrayify, requireInjection } from '@/utils/helpers'
import { PlayableListConfigKey, PlayableListSortFieldKey, PlayableListSortOrderKey } from '@/config/symbols'
import type { getPlayableCollectionContentType } from '@/utils/typeGuards'

import M3Chip from '@/components/m3/M3Chip.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import M3Menu from '@/components/m3/M3Menu.vue'
import M3MenuItem from '@/components/m3/M3MenuItem.vue'

const props = withDefaults(
  defineProps<{
    contentType?: ReturnType<typeof getPlayableCollectionContentType>
  }>(),
  {
    contentType: 'songs',
  },
)

const emit = defineEmits<{
  (e: 'sort', field: MaybeArray<PlayableListSortField>, order: SortOrder): void
}>()

const [sortField, setSortField] =
  requireInjection<[Ref<MaybeArray<PlayableListSortField>>, Closure]>(PlayableListSortFieldKey)
const [sortOrder, setSortOrder] = requireInjection<[Ref<SortOrder>, Closure]>(PlayableListSortOrderKey)
const [config] = requireInjection<[Partial<PlayableListConfig>]>(PlayableListConfigKey, [{}])

const open = ref(false)
const container = useTemplateRef('container')
onClickOutside(container, () => (open.value = false))

const options = computed<Array<{ label: string; field: MaybeArray<PlayableListSortField> }>>(() => {
  if (props.contentType === 'episodes') {
    return [
      { label: 'Title', field: 'title' },
      { label: 'Podcast', field: 'podcast_title' },
      { label: 'Author', field: 'podcast_author' },
      { label: 'Duration', field: 'length' },
    ]
  }

  return [
    { label: 'Title', field: 'title' },
    { label: 'Artist', field: 'artist_name' },
    { label: 'Album', field: 'album_name' },
    { label: 'Track', field: 'track' },
    { label: 'Year', field: 'year' },
    { label: 'Genre', field: 'genre' },
    { label: 'Duration', field: 'length' },
    { label: 'Date added', field: 'created_at' },
  ]
})

const isCurrent = (field: MaybeArray<PlayableListSortField>) =>
  arrayify(field).join() === arrayify(sortField.value).join()

const currentLabel = computed(() => options.value.find(({ field }) => isCurrent(field))?.label ?? 'Sort')

const sort = (field: MaybeArray<PlayableListSortField>) => {
  setSortOrder(isCurrent(field) && sortOrder.value === 'asc' ? 'desc' : 'asc')
  setSortField(field)
  open.value = false

  emit('sort', field, sortOrder.value)
}
</script>

<style scoped>
.sort-bar {
  position: relative;
  display: flex;
  justify-content: flex-end;
  padding: 0 24px 4px;
  z-index: 5;
}

.menu {
  position: absolute;
  top: calc(100% + 4px);
  right: 24px;
  min-width: 200px;
}
</style>
