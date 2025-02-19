<template>
  <div class="login-container">
    <div class="login">
      <div class="header">Welcome Back!</div>
      <form @submit.prevent="handleLogin" class="login-form">
        <div class="input-group">
          <input
            id="password-holder"
            type="password"
            v-model="password"
            placeholder="Password"
            required
          />
        </div>
        <button type="submit" class="login-button">Login</button>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import Cookies from 'js-cookie'
import axios from 'axios'
import { useRouter } from 'vue-router'
import { z } from 'zod'
import { useRedirectionStore } from '@/store/redirectionStore'
const password = ref('')
const router = useRouter()
const redirectionStore = useRedirectionStore('mainId')
const handleLogin = async () => {
  try {
    const response = await axios.post('/post/authenticate', JSON.stringify(password.value), {
      headers: {
        'Content-Type': 'application/json'
      }
    })

    // Validate response.data using Zod
    const tokenValue = z.string().parse(response.data) // Ensures response.data is a string

    // Store the JWT in a cookie with security attributes
    Cookies.set('jwt', tokenValue, {
      httpOnly: false, // Set to true for better security (cannot access via JavaScript)
      secure: true, // Ensure it's only sent over HTTPS
      sameSite: 'Strict', // Prevent CSRF attacks
      expires: 1 // Optional: Expires in 1 day
    })
    const redirection = redirectionStore.redirection
    if (redirection !== null) {
      await router.push(redirection)
    } else {
      await router.push({ name: 'home' })
    }
  } catch (error) {
    console.error('Error during login:', error)
  }
}
</script>

<style scoped>
.login-container {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100vh;
  background-color: #3d3d3d;
}

.login {
  max-width: 400px;
  width: 100%;
  padding: 40px;
  background-color: #3d3d3d;
  border: 1px solid gainsboro;
  border-radius: 10px;
  box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);
}

.header {
  font-size: 24px;
  margin-bottom: 20px;
  color: gainsboro;
  text-align: center;
}

.login-form {
  display: flex;
  flex-direction: column;
}

.input-group {
  margin-bottom: 20px;
}

input {
  box-sizing: border-box;
  font-size: 18px;
  border-radius: 5px;
  border: 1px solid #ccc;
  color: gainsboro;
  background-color: #2c2c2c;
  width: 100%;
  padding: 10px;
}

input::placeholder {
  color: #aaa;
}

.login-button {
  padding: 10px;
  font-size: 18px;
  background-color: #28a745;
  color: white;
  border: none;
  border-radius: 5px;
  cursor: pointer;
  transition: background-color 0.3s ease;
}

.login-button:hover {
  background-color: #218838;
}
</style>
