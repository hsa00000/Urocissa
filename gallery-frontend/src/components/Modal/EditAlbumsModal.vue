<template>
  <v-dialog
    v-if="submit !== undefined"
    v-model="modalStore.showEditAlbumsModal"
    variant="flat"
    persistent
    id="edit-album-overlay"
  >
    <v-confirm-edit
      v-model="changedAlbums"
      :disabled="false"
      @save="submit"
      @cancel="modalStore.showEditAlbumsModal = false"
    >
      <template #default="{ model: proxyModel, actions }">
        <v-card class="mx-auto w-100" max-width="400" variant="elevated" retain-focus>
          <template #title>Edit&nbsp;Albums</template>

          <template #text>
            <v-form v-model="formIsValid" @submit.prevent="submit" validate-on="input">
              <v-select
                v-model="proxyModel.value"
                chips
                multiple
                :items="albumItems"
                item-title="displayName"
                item-value="albumId"
                label="Albums"
                variant="outlined"
                closable-chips
                return-object
              />
            </v-form>
          </template>

          <v-divider />

          <template #actions>
            <v-spacer />
            <component :is="actions" />
          </template>
        </v-card>
      </template>
    </v-confirm-edit>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, toRaw } from 'vue'
import { useRoute } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { useAlbumStore } from '@/store/albumStore'
import type { AlbumInfo } from '@type/types'
import { getHashIndexDataFromRoute, getIsolationIdByRoute } from '@utils/getter'
import { editAlbums } from '@/api/editAlbums'

const formIsValid = ref(false)
const changedAlbums = ref<AlbumInfo[]>([])
const submit = ref<(() => Promise<void>) | undefined>()

const route = useRoute()
const modalStore = useModalStore('mainId')
const albumStore = useAlbumStore('mainId')

const albumItems = computed<AlbumInfo[]>(() =>
  [...albumStore.albums.values()].map((a) => structuredClone(toRaw(a)))
)

onMounted(() => {
  const initSubmit = (): (() => Promise<void>) | undefined => {
    const parsed = getHashIndexDataFromRoute(route)
    if (!parsed) {
      console.error('initSubmit: failed to parse route.')
      return
    }

    const { index, data } = parsed
    if (data.type !== 'image' && data.type !== 'video') {
      console.error("initSubmit: 'data' is not an image or video.")
      return
    }

    const defaultAlbumIds = [...data.albums]

    const initialAlbums = defaultAlbumIds
      .map((id) => albumStore.albums.get(id))
      .filter((a): a is AlbumInfo => a !== undefined)
      .map((a) => structuredClone(toRaw(a)))
    // Temporary workaround: VConfirmEdit internally uses structuredClone,
    // which cannot clone Vue reactive proxies directly.
    // We use toRaw() + structuredClone() here to ensure plain objects.

    changedAlbums.value = initialAlbums

    const innerSubmit = async () => {
      const selectedIds = changedAlbums.value.map((a) => a.albumId)

      const addAlbumIds = selectedIds.filter((id) => !defaultAlbumIds.includes(id))
      const removeAlbumIds = defaultAlbumIds.filter((id) => !selectedIds.includes(id))

      await editAlbums([index], addAlbumIds, removeAlbumIds, getIsolationIdByRoute(route))

      modalStore.showEditAlbumsModal = false
    }

    return innerSubmit
  }

  submit.value = initSubmit()
})
</script>
