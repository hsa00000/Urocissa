import 'vue-router'
import { createRouter, createWebHistory, RouteRecordRaw } from 'vue-router'
import { simpleRoutes } from '@/route/simpleRoutes'
import { shareRoute } from '@/route/shareRoutes'
import { virtualRoutes } from '@/route/virtualRoutes'
import { redirectionRoutes } from '@/route/redirectionRoutes'

const routes: RouteRecordRaw[] = [
  ...simpleRoutes,
  ...virtualRoutes,
  ...shareRoute,
  ...redirectionRoutes
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

export default router
