// src/router.ts

import { Component } from 'vue'
import { createRouter, createWebHistory, LocationQuery, RouteRecordRaw } from 'vue-router'
import 'vue-router'

import TagsPage from '@/components/Page/TagsPage.vue'
import LoginPage from '@/components/LoginPage.vue'
import HomeMain from '@/components/Home/Page/HomeMain.vue'
import AllPage from '@/components/Page/AllPage.vue'
import FavoritePage from '@/components/Page/FavoritePage.vue'
import ArchivedPage from '@/components/Page/ArchivedPage.vue'
import TrashedPage from '@/components/Page/TrashedPage.vue'
import AlbumsPage from '@/components/Page/AlbumsPage.vue'
import ViewPageMain from '@/components/Home/View/Page/ViewPageMain.vue'
import HomeIsolated from '@/components/Home/Page/HomeIsolated.vue'
import ViewPageIsolated from '@/components/Home/View/Page/ViewPageIsolated.vue'
import VideosPage from '@/components/Page/VideosPage.vue'
import HomeShare from '@/components/Home/Page/HomeShare.vue'

interface PageReturnType {
  name: string
  params: { hash: string | string[] | undefined; subhash: string | string[] | undefined }
  query: LocationQuery
}

type BaseName =
  | 'home'
  | 'all'
  | 'favorite'
  | 'archived'
  | 'trashed'
  | 'albums'
  | 'videos'
  | 'album'
  | 'tags'
  | 'login'
  | 'share'

// ======================================
// 1. Define Simple Static Routes
// ======================================

const simpleRoutes: RouteRecordRaw[] = [
  { path: '/', redirect: '/home' },
  {
    path: '/tags',
    component: TagsPage,
    name: 'TagsPage',
    meta: {
      isReadPage: false,
      isViewPage: false,
      basicString: null,
      baseName: 'tags',
      getParentPage: (route) => {
        return {
          name: 'home',
          params: { hash: undefined, subhash: undefined },
          query: route.query
        }
      },
      getChildPage: (route) => {
        return {
          name: 'TagsPage',
          params: { hash: undefined, subhash: undefined },
          query: route.query
        }
      }
    }
  },
  {
    path: '/login',
    component: LoginPage,
    name: 'LoginPage',
    meta: {
      isReadPage: false,
      isViewPage: false,
      basicString: null,
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
          name: 'LoginPage',
          params: { hash: undefined, subhash: undefined },
          query: route.query
        }
      }
    }
  },
  {
    path: '/share-:shareId',
    component: HomeShare,
    name: 'SharePage',
    meta: {
      isReadPage: false,
      isViewPage: false,
      basicString: null,
      baseName: 'share'
    }
  }
]

// ======================================
// 2. Define a Helper Function to Create Routes
// ======================================

/**
 * Creates a main route with an optional child route.
 *
 * @param path - The base path for the route.
 * @param component - The component to be rendered.
 * @param name - The unique name for the route.
 * @returns An array containing the RouteRecordRaw object.
 */
function createRoute(
  baseName: BaseName,
  component: Component,
  basicString: string | null
): RouteRecordRaw[] {
  const mainRoute: RouteRecordRaw = {
    path: `/${baseName}`,
    component: component,
    name: baseName,
    meta: {
      isReadPage: false,
      isViewPage: false,
      basicString: basicString,
      baseName: baseName,
      getParentPage: (route) => {
        return {
          name: baseName,
          params: { hash: undefined, subhash: undefined },
          query: route.query
        }
      },
      getChildPage: (route, hash) => {
        return {
          name: `${baseName}ViewPage`,
          params: { hash: hash, subhash: undefined },
          query: route.query
        }
      }
    },
    children: [
      {
        path: 'view/:hash',
        component: ViewPageMain,
        name: `${baseName}ViewPage`,
        meta: {
          isReadPage: false,
          isViewPage: true,
          basicString: basicString,
          baseName: baseName,
          getParentPage: (route) => {
            return {
              name: baseName,
              params: { hash: undefined, subhash: undefined },
              query: route.query
            }
          },
          getChildPage: (route) => {
            return {
              name: `${baseName}ReadPage`,
              params: { hash: route.params.hash, subhash: undefined },
              query: route.query
            }
          }
        },
        children: [
          {
            path: 'read',
            component: HomeIsolated,
            name: `${baseName}ReadPage`,
            meta: {
              isReadPage: true,
              isViewPage: false,
              basicString: basicString,
              baseName: baseName,
              getParentPage: (route) => {
                return {
                  name: `${baseName}ViewPage`,
                  params: { hash: route.params.hash, subhash: undefined },
                  query: route.query
                }
              },
              getChildPage: (route, subhash) => {
                return {
                  name: `${baseName}ReadViewPage`,
                  params: { hash: route.params.hash, subhash: subhash },
                  query: route.query
                }
              }
            },
            children: [
              {
                path: 'view/:subhash',
                name: `${baseName}ReadViewPage`,
                component: ViewPageIsolated,
                meta: {
                  isReadPage: true,
                  isViewPage: true,
                  basicString: basicString,
                  baseName: baseName,
                  getParentPage: (route) => {
                    return {
                      name: `${baseName}ReadPage`,
                      params: { hash: route.params.hash, subhash: undefined },
                      query: route.query
                    }
                  },
                  getChildPage: (route) => {
                    return {
                      name: `${baseName}ReadViewPage`,
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
  return [mainRoute]
}

// ======================================
// 3. Create Routes Using the Helper Function
// ======================================

const homePageRoutes = createRoute(
  'home',
  HomeMain,
  'and(not(tag:"_archived"), not(tag:"_trashed"))'
)

const allPageRoutes = createRoute('all', AllPage, 'not(tag:"_trashed")')

const favoritePageRoutes = createRoute(
  'favorite',
  FavoritePage,
  'and(tag:"_favorite", not(tag:"_trashed"))'
)

const archivedPageRoutes = createRoute(
  'archived',
  ArchivedPage,
  'and(tag:"_archived", not(tag:"_trashed"))'
)

const trashedPageRoutes = createRoute('trashed', TrashedPage, 'and(tag:"_trashed")')

const albumsPageRoutes = createRoute('albums', AlbumsPage, 'and(type:"album", not(tag:"_trashed"))')

const videosPageRoutes = createRoute(
  'videos',
  VideosPage,
  'and(type:"video", not(tag:"_archived"), not(tag:"_trashed"))'
)

// ======================================
// 4. Combine All Routes
// ======================================

const routes: RouteRecordRaw[] = [
  ...simpleRoutes,
  ...homePageRoutes,
  ...allPageRoutes,
  ...favoritePageRoutes,
  ...archivedPageRoutes,
  ...trashedPageRoutes,
  ...albumsPageRoutes,
  ...videosPageRoutes
]

// ======================================
// 5. Create and Export the Router Instance
// ======================================

const router = createRouter({
  history: createWebHistory(),
  routes
})

declare module 'vue-router' {
  interface RouteMeta {
    isReadPage: boolean
    isViewPage: boolean
    baseName: BaseName
    basicString: string | null
    getParentPage: (router: RouteLocationNormalizedLoadedGeneric) => PageReturnType
    getChildPage: (router: RouteLocationNormalizedLoadedGeneric, hash: string) => PageReturnType
  }
}

export default router
