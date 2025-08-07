// src/router.ts

import { createRouter, createWebHistory, RouteRecordRaw } from 'vue-router'
import 'vue-router'

import UniversalPage from '@/components/Page/UniversalPage.vue'
import ViewPageMain from '@/components/View/ViewPageMain.vue'
import HomeIsolated from '@/components/Home/HomeIsolated.vue'
import ViewPageIsolated from '@/components/View/ViewPageIsolated.vue'
import { tagsRoute } from './tagsRoute'
import { linksRoute } from './linksRoute'
import { loginRoute } from './loginRoute'
import { shareRoute } from './shareRoute'

// ======================================
// Define Simple Static Routes
// ======================================

const simpleRoutes: RouteRecordRaw[] = [
  { path: '/', redirect: '/gallery?type=home' },
  tagsRoute,
  linksRoute,
  loginRoute,
  shareRoute
]

// ======================================
// Define Universal Gallery Route
// ======================================

const galleryRoute: RouteRecordRaw = {
  path: '/gallery',
  component: UniversalPage,
  name: 'gallery',
  meta: {
    isReadPage: false,
    isViewPage: false,
    baseName: 'gallery',
    getParentPage: (route) => {
      return {
        name: 'gallery',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    },
    getChildPage: (route, hash) => {
      return {
        name: 'galleryViewPage',
        params: { hash: hash, subhash: undefined },
        query: route.query
      }
    }
  },
  children: [
    {
      path: 'view/:hash',
      component: ViewPageMain,
      name: 'galleryViewPage',
      meta: {
        isReadPage: false,
        isViewPage: true,
        baseName: 'gallery',
        getParentPage: (route) => {
          return {
            name: 'gallery',
            params: { hash: undefined, subhash: undefined },
            query: route.query
          }
        },
        getChildPage: (route) => {
          return {
            name: 'galleryReadPage',
            params: { hash: route.params.hash, subhash: undefined },
            query: route.query
          }
        }
      },
      children: [
        {
          path: 'read',
          component: HomeIsolated,
          name: 'galleryReadPage',
          meta: {
            isReadPage: true,
            isViewPage: false,
            baseName: 'gallery',
            getParentPage: (route) => {
              return {
                name: 'galleryViewPage',
                params: { hash: route.params.hash, subhash: undefined },
                query: route.query
              }
            },
            getChildPage: (route, subhash) => {
              return {
                name: 'galleryReadViewPage',
                params: { hash: route.params.hash, subhash: subhash },
                query: route.query
              }
            }
          },
          children: [
            {
              path: 'view/:subhash',
              name: 'galleryReadViewPage',
              component: ViewPageIsolated,
              meta: {
                isReadPage: true,
                isViewPage: true,
                baseName: 'gallery',
                getParentPage: (route) => {
                  return {
                    name: 'galleryReadPage',
                    params: { hash: route.params.hash, subhash: undefined },
                    query: route.query
                  }
                },
                getChildPage: (route) => {
                  return {
                    name: 'galleryReadViewPage',
                    params: { hash: route.params.hash, subhash: route.params.subhash },
                    query: route.query
                  }
                }
              }
            }
          ]
        }
      ]
    }
  ]
}

// 为了兼容性，添加重定向路由
const compatibilityRoutes: RouteRecordRaw[] = [
  { path: '/home', redirect: (to) => ({ path: '/gallery', query: { ...to.query, type: 'home' } }) },
  { path: '/all', redirect: (to) => ({ path: '/gallery', query: { ...to.query, type: 'all' } }) },
  {
    path: '/favorite',
    redirect: (to) => ({ path: '/gallery', query: { ...to.query, type: 'favorite' } })
  },
  {
    path: '/archived',
    redirect: (to) => ({ path: '/gallery', query: { ...to.query, type: 'archived' } })
  },
  {
    path: '/trashed',
    redirect: (to) => ({ path: '/gallery', query: { ...to.query, type: 'trashed' } })
  },
  {
    path: '/albums',
    redirect: (to) => ({ path: '/gallery', query: { ...to.query, type: 'albums' } })
  },
  {
    path: '/videos',
    redirect: (to) => ({ path: '/gallery', query: { ...to.query, type: 'videos' } })
  }
]

// ======================================
// Combine All Routes
// ======================================

const routes: RouteRecordRaw[] = [...simpleRoutes, galleryRoute, ...compatibilityRoutes]

// ======================================
// Create and Export the Router Instance
// ======================================

const router = createRouter({
  history: createWebHistory(),
  routes
})

export default router
