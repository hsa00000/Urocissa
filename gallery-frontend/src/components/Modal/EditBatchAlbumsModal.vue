<template>
  <v-dialog
    v-if="submit !== undefined"
    v-model="modalStore.showBatchEditAlbumsModal"
    persistent
    id="batch-edit-album-overlay"
    max-width="400"
  >
    <v-confirm-edit
      v-model="changedAlbums"
      :disabled="false"
      @save="submit"
      @cancel="modalStore.showBatchEditAlbumsModal = false"
    >
      <template #default="{ model: proxyModel, actions }">
        <v-card variant="elevated" retain-focus>
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
                      <template #prepend="{ isActive }">
                        <v-list-item-action>
                          <v-checkbox-btn
                            v-if="!loading"
                            :model-value="isActive"
                            false-icon="mdi-plus"
                            true-icon="mdi-plus"
                            indeterminate-icon="mdi-plus"
                          />
                          <v-checkbox-btn
                            v-else
                            :model-value="isActive"
                            false-icon="mdi-loading"
                            true-icon="mdi-loading"
                            indeterminate-icon="mdi-loading"
                          />
                        </v-list-item-action>

                        <v-list-item-title class="wrap"> Create New Album </v-list-item-title>
                      </template>
                    </v-list-item>
                    <v-divider />
                  </template>

                  <template #no-data v-else>
                    <v-list-item value="" @click.stop.prevent="createNonEmptyAlbumWithLoading">
                      <template #prepend="{ isActive }">
                        <v-list-item-action>
                          <v-checkbox-btn
                            v-if="!loading"
                            :model-value="isActive"
                            false-icon="mdi-plus"
                            true-icon="mdi-plus"
                            indeterminate-icon="mdi-plus"
                          />
                          <v-checkbox-btn
                            v-else
                            :model-value="isActive"
                            false-icon="mdi-loading"
                            true-icon="mdi-loading"
                            indeterminate-icon="mdi-loading"
                          />
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
import { useRoute } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { useCollectionStore } from '@/store/collectionStore'
import { useAlbumStore } from '@/store/albumStore'
import { getIsolationIdByRoute } from '@utils/getter'
import { useCreateAlbumAction } from '@/script/hook/useCreateAlbumAction'
import type { AlbumInfo } from '@type/types'
import type { VForm } from 'vuetify/components'
import { editAlbums } from '@/api/editAlbums'

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)

const modalStore = useModalStore('mainId')
const collectionStore = useCollectionStore(isolationId)
const albumStore = useAlbumStore(isolationId)
const { loading, createAndNavigate } = useCreateAlbumAction()

const formRef = ref<VForm | null>(null)
const formIsValid = ref(false)

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
    await editAlbums(
      collectionStore.descriptor(),
      changedAlbums.value.add.map((a) => a.albumId),
      changedAlbums.value.remove.map((a) => a.albumId),
      isolationId
    )

    modalStore.showBatchEditAlbumsModal = false
  }
})

const createNonEmptyAlbumWithLoading = async () => {
  await createAndNavigate(collectionStore.descriptor(), isolationId, () => {
    modalStore.showBatchEditAlbumsModal = false
    collectionStore.editModeOn = false
  })
}

watch(
  () => [changedAlbums.value.add, changedAlbums.value.remove],
  async () => {
    await formRef.value?.validate()
  },
  { deep: true }
)
</script>
