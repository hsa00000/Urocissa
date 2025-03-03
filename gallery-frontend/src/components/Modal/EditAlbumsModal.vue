<template>
  <v-dialog
    v-if="submit !== undefined"
    v-model="modalStore.showEditAlbumsModal"
    variant="flat"
    persistent
    id="edit-album-overlay"
  >
    <v-card class="mx-auto w-100" max-width="400" variant="elevated" retain-focus>
      <v-form v-model="formIsValid" @submit.prevent="submit" validate-on="input">
        <v-card-title> Edit Albums </v-card-title>
        <v-container>
          <!-- 
  v-model="reactiveArray":
    - Binds to the list (reactiveArray) of selected objects.
    - Choose between `return-object` or `item-value`:
      - `return-object`: reactiveArray will be the list of object.
      - `item-value`: reactiveArray will be the list of object.value
      - Rmk. In this case `item-value` you can further assign item-value="myValue" so that reactiveArray will be the list of object.myValue

  items:
    - The list of objects that can be selected by the user.

  item-title:
    - If set to "field", displays `object.field` to the user (in option filed)

  label:
    - If set to "SomeText", displays "SomeText" to the user (in text field)
-->
          <v-select
            v-model="vModelAlbumsArray"
            chips
            multiple
            item-title="displayName"
            :items="[...albumStore.albums.values()]"
            label="Albums"
            closable-chips
            return-object
          ></v-select>
        </v-container>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn
            color="teal-accent-4"
            variant="outlined"
            class="ma-2 button button-submit"
            @click="modalStore.showEditAlbumsModal = false"
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
 * This modal is used for editing the albums of a single photo on the single photo view page.
 */
import { ref, onMounted, watchEffect } from 'vue'
import { useRoute } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { useAlbumStore } from '@/store/albumStore'
import { editAlbumsInWorker } from '@/script/inWorker/editAlbumsInWorker'
import { AlbumInfo } from '@/script/common/types'
import { getHashIndexDataFromRoute, getIsolationIdByRoute } from '@utils/getter'

const route = useRoute()

const modalStore = useModalStore('mainId')
const albumStore = useAlbumStore('mainId')

const formIsValid = ref(false)
const vModelAlbumsArray = ref<AlbumInfo[]>([])

const submit = ref<(() => void) | undefined>(undefined)

onMounted(() => {
  const useSubmit = (): undefined | (() => void) => {
    const initializeResult = getHashIndexDataFromRoute(route)
    if (initializeResult === undefined) {
      console.error(
        "useSubmit Error: Failed to initialize result. 'getHashIndexDataFromRoute(route)' returned undefined."
      )
      return undefined
    }

    const { index, data } = initializeResult
    if (data.database === undefined) {
      console.error("useSubmit Error: 'data.database' is undefined.")
      return undefined
    }

    const defaultAlbums: AlbumInfo[] = []

    for (const albumId of data.database.album) {
      const albumName = albumStore.albums.get(albumId)
      if (albumName === undefined) {
        console.error(`useSubmit Error: Album name not found for albumId '${albumId}'.`)
        return undefined
      }
      defaultAlbums.push(albumName)
    }

    // Initialize vModelAlbumsArray with defaultAlbums
    vModelAlbumsArray.value = defaultAlbums

    const innerSubmit = () => {
      // Hash of the current photo/video
      const idArray: number[] = [index]

      // Albums to be added: present in vModelAlbumsArray but not in defaultAlbums
      const addAlbumsArrayComputed = vModelAlbumsArray.value.filter(
        (album) => !defaultAlbums.some((defaultAlbum) => defaultAlbum.albumId === album.albumId)
      )

      // Albums to be removed: present in defaultAlbums but not in vModelAlbumsArray
      const removeAlbumsArrayComputed = defaultAlbums.filter(
        (defaultAlbum) =>
          !vModelAlbumsArray.value.some((album) => album.albumId === defaultAlbum.albumId)
      )

      editAlbumsInWorker(
        idArray,
        addAlbumsArrayComputed.map((album) => album.albumId),
        removeAlbumsArrayComputed.map((album) => album.albumId),
        getIsolationIdByRoute(route)
      )
      modalStore.showEditAlbumsModal = false
    }

    return innerSubmit
  }

  submit.value = useSubmit()
})

watchEffect(() => {
  console.log('vModelAlbumsArray is', vModelAlbumsArray.value)
})
</script>
