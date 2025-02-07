<template>
  <v-dialog v-model="modalStore.showShareModal" id="share-modal" variant="flat" persistent style="">
    <v-card class="h-100 mx-auto w-100" max-width="400" variant="elevated" retain-focus>
      <v-form>
        <v-toolbar color="transparent">
          <v-toolbar-title class="text-h5" text=" Share"></v-toolbar-title>
          <template #append>
            <v-btn icon="mdi-dots-vertical"></v-btn>
          </template>
        </v-toolbar>
        <v-list lines="one">
          <v-list-item>
            <v-switch
              v-model="requirePassword"
              color="primary"
              :label="`Password: ${requirePassword}`"
              hide-details
            ></v-switch>
            <v-text-field label="Label"></v-text-field>
            <v-switch
              v-model="willExpire"
              color="primary"
              :label="`Expire: ${willExpire}`"
              hide-details
            ></v-switch>
            <v-switch
              v-model="allowUpload"
              color="primary"
              :label="`allowUpload: ${allowUpload}`"
              hide-details
            ></v-switch>
          </v-list-item>
        </v-list>
      </v-form>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { useModalStore } from '@/store/modalStore'
import { ref } from 'vue'
const modalStore = useModalStore('mainId')
const requirePassword = ref(false)
const willExpire = ref(false)
const allowUpload = ref(false)
</script>
