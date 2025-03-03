<template>
  <v-dialog
    v-model="modalStore.showBatchEditAlbumsModal"
    variant="flat"
    persistent
    id="batch-edit-album-overlay"
  >
    <v-card class="mx-auto w-100" max-width="400" variant="elevated">
      <v-card-title class="text-h5"> Edit Albums </v-card-title>
      <v-form ref="formRef" v-model="formIsValid" @submit.prevent="submit">
        <v-container>
          <v-combobox
            clearable
            v-model="addAlbumsArray"
            chips
            multiple
            label="Add to Albumss"
            item-title="displayName"
            :rules="[addAlbumsRule]"
            :items="[...albumStore.albums.values()]"
            item-value="albumId"
            :hide-no-data="false"
            return-object
            :menu-props="{ maxWidth: 0 }"
            closable-chips
          >
            <template #prepend-item v-if="albumStore.albums.size > 0">
              <v-list-item value="">
                <template #prepend>
                  <v-list-item-action>
                    <v-btn
                      v-if="!loading"
                      color="transparent"
                      icon="mdi-plus"
                      density="comfortable"
                      flat
                    ></v-btn>
                    <v-btn v-else color="transparent" icon density="comfortable" flat
                      ><v-progress-circular size="24" indeterminate></v-progress-circular
                    ></v-btn>
                  </v-list-item-action>
                  <v-list-item-title class="wrap" @click="createNonEmptyAlbum()"
                    >Create New Album</v-list-item-title
                  >
                </template>
              </v-list-item>
              <v-divider></v-divider>
            </template>

            <template #no-data v-else>
              <v-list-item value="">
                <template #prepend>
                  <v-list-item-action>
                    <v-btn color="transparent" icon="mdi-plus" density="comfortable" flat></v-btn>
                  </v-list-item-action>
                  <v-list-item-title class="wrap" @click="createNonEmptyAlbum()"
                    >Create New Album</v-list-item-title
                  >
                </template>
              </v-list-item>
              <v-divider></v-divider>
            </template>
          </v-combobox>
        </v-container>
        <v-container>
          <v-combobox
            v-model="removeAlbumsArray"
            chips
            multiple
            label="Remove from Albums"
            item-title="displayName"
            :rules="[removeAlbumsRule]"
            :items="[...albumStore.albums.values()]"
            item-value="albumId"
            return-objects
            :menu-props="{ maxWidth: 0 }"
            closable-chips
          ></v-combobox>
        </v-container>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn
            color="teal-accent-4"
            variant="outlined"
            class="ma-2 button button-submit"
            @click="modalStore.showBatchEditAlbumsModal = false"
          >
            Cancel
          </v-btn>
          <v-btn
            color="teal-accent-4"
            variant="outlined"
            class="ma-2 button button-submit"
            :disabled="!formIsValid"
            type="submit"
          >
            Submit
          </v-btn>
        </v-card-actions>
      </v-form>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
/**
 * This modal is used for editing the albums of multiple photos on the home page.
 */
import { ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { useCollectionStore } from '@/store/collectionStore'
import { useAlbumStore } from '@/store/albumStore'
import { AlbumInfo } from '@/script/common/types'
import { editAlbumsInWorker } from '@/script/inWorker/editAlbumsInWorker'
import { getIsolationIdByRoute } from '@utils/getter'
import { createAlbum } from '@/script/common/createAlbums'
import { navigateToAlbum } from '@/script/navigator'
import { VForm } from 'vuetify/components/VForm'
const route = useRoute()
const router = useRouter()
const isolationId = getIsolationIdByRoute(route)

const formRef = ref<VForm | null>(null)
const formIsValid = ref(false)
const loading = ref(false)

const collectionStore = useCollectionStore(isolationId)
const albumStore = useAlbumStore(isolationId)
const modalStore = useModalStore('mainId')

const addAlbumsArray = ref<AlbumInfo[]>([])
const removeAlbumsArray = ref<AlbumInfo[]>([])

// Rule for Add Albums to ensure no album is added that's already in Remove Albums
const addAlbumsRule = (inputArray: AlbumInfo[]) => {
  return (
    inputArray.every(
      (album) =>
        !removeAlbumsArray.value.map((removeAlbum) => removeAlbum.albumId).includes(album.albumId)
    ) || 'Some albums are already selected in Remove Albums'
  )
}

// Rule for Remove Albums to ensure no album is added that's already in Add Albums
const removeAlbumsRule = (inputArray: AlbumInfo[]) =>
  inputArray.every(
    (album) => !addAlbumsArray.value.map((addAlbum) => addAlbum.albumId).includes(album.albumId)
  ) || 'Some albums are already selected in Add Albums'

const submit = () => {
  const hashArray = Array.from(collectionStore.editModeCollection)
  editAlbumsInWorker(
    hashArray,
    addAlbumsArray.value.map((album) => album.albumId),
    removeAlbumsArray.value.map((album) => album.albumId),
    isolationId
  )
  modalStore.showBatchEditAlbumsModal = false
}

const createNonEmptyAlbum = async () => {
  loading.value = true
  const newAlbumId = await createAlbum([...collectionStore.editModeCollection], isolationId)
  if (typeof newAlbumId === 'string') {
    await navigateToAlbum(newAlbumId, router)
    modalStore.showBatchEditAlbumsModal = false
    collectionStore.editModeOn = false
    loading.value = false
  }
}

watch([addAlbumsArray, removeAlbumsArray], async () => {
  await formRef.value?.validate()
})
</script>
