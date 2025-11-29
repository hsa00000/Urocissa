// src/router.ts

import { RouteRecordRaw } from 'vue-router'
import 'vue-router'

import LoginPage from '@/components/Login/LoginPage.vue'

export const loginRoute: RouteRecordRaw = {
  path: '/login',
  component: LoginPage,
  name: 'login',

  meta: {
    basicString: null,
    level: 1,
    baseName: 'login',
    getParentPage: (route) => {
      return {
        name: 'home',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    },
    getChildPage: (route) => {
      return {
        name: 'login',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    }
  }
}
