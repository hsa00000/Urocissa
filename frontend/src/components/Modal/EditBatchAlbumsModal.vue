<template>
  <v-dialog
    v-if="submit !== undefined"
    v-model="modalStore.showBatchEditAlbumsModal"
    variant="flat"
    persistent
    id="batch-edit-album-overlay"
  >
    <v-confirm-edit
      v-model="changedAlbums"
      :disabled="false"
      @save="submit"
      @cancel="modalStore.showBatchEditAlbumsModal = false"
    >
      <template #default="{ model: proxyModel, actions }">
        <v-card class="mx-auto w-100" max-width="400" variant="elevated" retain-focus>
          <template #title>Edit&nbsp;Albums</template>
          <template #text>
            <v-form
              ref="formRef"
              v-model="formIsValid"
              @submit.prevent="submit"
              validate-on="input"
            >
              <v-container>
                <v-combobox
                  clearable
                  v-model="proxyModel.value.add"
                  chips
                  multiple
                  item-title="displayName"
                  item-value="albumId"
                  :items="albumItems"
                  :rules="[addAlbumsRule]"
                  label="Add to Albums"
                  return-object
                  closable-chips
                  :menu-props="{ maxWidth: 0 }"
                  :hide-no-data="false"
                  autocomplete="off"
                >
                  <template #prepend-item v-if="albumStore.albums.size > 0">
                    <v-list-item value="" @click.stop.prevent="createNonEmptyAlbumWithLoading">
                      <template #prepend>
                        <v-list-item-action>
                          <v-btn
                            v-if="!loading"
                            color="transparent"
                            icon="mdi-plus"
                            density="comfortable"
                            flat
                          />
                          <v-btn v-else color="transparent" icon density="comfortable" flat>
                            <v-progress-circular indeterminate size="24" />
                          </v-btn>
                        </v-list-item-action>
                        <v-list-item-title class="wrap"> Create New Album </v-list-item-title>
                      </template>
                    </v-list-item>
                    <v-divider />
                  </template>

                  <template #no-data v-else>
                    <v-list-item value="" @click.stop.prevent="createNonEmptyAlbumWithLoading">
                      <template #prepend>
                        <v-list-item-action>
                          <v-btn color="transparent" icon="mdi-plus" density="comfortable" flat />
                        </v-list-item-action>
                        <v-list-item-title class="wrap"> Create New Album </v-list-item-title>
                      </template>
                    </v-list-item>
                  </template>
                </v-combobox>
              </v-container>

              <v-container>
                <v-combobox
                  v-model="proxyModel.value.remove"
                  chips
                  multiple
                  item-title="displayName"
                  item-value="albumId"
                  :items="albumItems"
                  :rules="[removeAlbumsRule]"
                  label="Remove from Albums"
                  return-object
                  closable-chips
                  :menu-props="{ maxWidth: 0 }"
                  autocomplete="off"
                />
              </v-container>
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
import { ref, watch, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { isNavigationFailure, NavigationFailureType } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { useCollectionStore } from '@/store/collectionStore'
import { useAlbumStore } from '@/store/albumStore'
import { getIsolationIdByRoute } from '@utils/getter'
import { createNonEmptyAlbum } from '@utils/createAlbums'
import { navigateToAlbum } from '@/route/navigator'
import type { AlbumInfo } from '@type/types'
import type { VForm } from 'vuetify/components'
import { editAlbums } from '@/api/editAlbums'

const route = useRoute()
const router = useRouter()
const isolationId = getIsolationIdByRoute(route)

const modalStore = useModalStore('mainId')
const collectionStore = useCollectionStore(isolationId)
const albumStore = useAlbumStore(isolationId)

const formRef = ref<VForm | null>(null)
const formIsValid = ref(false)
const loading = ref(false)

interface ChangedAlbums {
  add: AlbumInfo[]
  remove: AlbumInfo[]
}
const changedAlbums = ref<ChangedAlbums>({ add: [], remove: [] })

const albumItems = computed<AlbumInfo[]>(() => [...albumStore.albums.values()])

const addAlbumsRule = (inputArray: AlbumInfo[]) =>
  inputArray.every(
    (album) => !changedAlbums.value.remove.map((a) => a.albumId).includes(album.albumId)
  ) || 'Some albums are already selected in Remove Albums'

const removeAlbumsRule = (inputArray: AlbumInfo[]) =>
  inputArray.every(
    (album) => !changedAlbums.value.add.map((a) => a.albumId).includes(album.albumId)
  ) || 'Some albums are already selected in Add Albums'

const submit = ref<(() => Promise<void>) | undefined>()

onMounted(() => {
  submit.value = async () => {
    const hashArray = Array.from(collectionStore.editModeCollection)

    await editAlbums(
      hashArray,
      changedAlbums.value.add.map((a) => a.albumId),
      changedAlbums.value.remove.map((a) => a.albumId),
      isolationId
    )

    modalStore.showBatchEditAlbumsModal = false
  }
})

const createNonEmptyAlbumWithLoading = async () => {
  loading.value = true
  try {
    const newAlbumId = await createNonEmptyAlbum(
      [...collectionStore.editModeCollection],
      isolationId
    )

    if (typeof newAlbumId === 'string' && newAlbumId.length > 0) {
      // 先把可能會擋導航的狀態關掉
      modalStore.showBatchEditAlbumsModal = false
      collectionStore.editModeOn = false

      try {
        const failure = await navigateToAlbum(newAlbumId, router)

        if (isNavigationFailure(failure, NavigationFailureType.aborted)) {
          console.warn('Navigation aborted:', failure)
        } else if (isNavigationFailure(failure, NavigationFailureType.cancelled)) {
          console.warn('Navigation cancelled:', failure)
        }
      } catch (err) {
        console.error('navigateToAlbum threw:', err)
      }
    }
  } finally {
    loading.value = false
  }
}

watch(
  () => [changedAlbums.value.add, changedAlbums.value.remove],
  async () => {
    await formRef.value?.validate()
  },
  { deep: true }
)
</script>
