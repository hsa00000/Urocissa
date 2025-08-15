<template>
  <v-app>
    <v-main>
      <v-container class="fill-height" fluid>
        <v-row align="center" justify="center">
          <v-col cols="12" sm="8" md="5" lg="4">
            <v-card class="pa-6" elevation="10">
              <v-row class="mb-4" align="center" justify="center">
                <v-avatar size="56" class="mb-2" color="primary">
                  <v-icon size="32">mdi-lock-outline</v-icon>
                </v-avatar>
              </v-row>

              <v-card-title class="justify-center">Welcome back</v-card-title>

              <v-card-text>
                <v-form ref="formRef" @submit.prevent="handleLogin">
                  <v-text-field
                    v-model="password"
                    :type="showPassword ? 'text' : 'password'"
                    label="Password"
                    required
                    prepend-inner-icon="mdi-key-variant"
                    :append-inner-icon="showPassword ? 'mdi-eye-off' : 'mdi-eye'"
                    @click:append-inner="toggleShowPassword"
                    autocomplete="current-password"
                  />

                  <v-row class="mt-4" justify="center">
                    <v-col cols="12">
                      <v-btn type="submit" color="primary" class="ma-0" block>
                        Sign in
                      </v-btn>
                    </v-col>
                  </v-row>
                </v-form>
              </v-card-text>

              <v-card-actions class="justify-center">
                <small class="text--secondary">Powered by Urocissa</small>
              </v-card-actions>
            </v-card>
          </v-col>
        </v-row>
      </v-container>
    </v-main>
  </v-app>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import Cookies from 'js-cookie'
import axios from 'axios'
import { useRouter } from 'vue-router'
import { z } from 'zod'
import { useRedirectionStore } from '@/store/redirectionStore'
import { tryWithMessageStore } from '@/script/utils/try_catch'

const password = ref('')
const showPassword = ref(false)
const formRef = ref()
const router = useRouter()
const redirectionStore = useRedirectionStore('mainId')

function toggleShowPassword() {
  showPassword.value = !showPassword.value
}

const handleLogin = async () => {
  await tryWithMessageStore('mainId', async () => {
    // Simple form guard
    if (!password.value || password.value.trim() === '') return

    const response = await axios.post('/post/authenticate', JSON.stringify(password.value), {
      headers: {
        'Content-Type': 'application/json'
      }
    })

    const tokenValue = z.string().parse(response.data)

    Cookies.set('jwt', tokenValue, {
      httpOnly: false,
      secure: true,
      sameSite: 'Strict',
      expires: 14
    })

    const redirection = redirectionStore.redirection
    if (redirection !== null) {
      await router.push(redirection)
    } else {
      await router.push({ name: 'home' })
    }
  })
}
</script>

<style scoped>
/* Minimal local tweaks; Vuetify handles the bulk of styling */
.v-application {
  background: linear-gradient(180deg, #1e1e1e 0%, #2b2b2b 100%);
}

.v-card {
  border-radius: 12px;
}
</style>
