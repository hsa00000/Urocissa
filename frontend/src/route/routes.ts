// src/router.ts

import { createRouter, createWebHistory, RouteRecordRaw } from 'vue-router'
import type { RouteLocationRaw } from 'vue-router'
import 'vue-router'

import HomeMain from '@/components/Page/HomePage.vue'
import AllPage from '@/components/Page/AllPage.vue'
import FavoritePage from '@/components/Page/FavoritePage.vue'
import ArchivedPage from '@/components/Page/ArchivedPage.vue'
import TrashedPage from '@/components/Page/TrashedPage.vue'
import AlbumsPage from '@/components/Page/AlbumsPage.vue'
import VideosPage from '@/components/Page/VideosPage.vue'
import { createRoute } from './createRoute'
import { tagsRoute } from './tagsRoute'
import { linksRoute } from './linksRoute'
import { loginRoute } from './loginRoute'
import { shareRoute } from './shareRoute'

// ======================================
// Define Simple Static Routes
// ======================================

const simpleRoutes: RouteRecordRaw[] = [
  { path: '/', redirect: '/home' },
  tagsRoute,
  linksRoute,
  loginRoute,
  shareRoute
]

// ======================================
// Create Routes Using the Helper Function
// ======================================

const homePageRoutes = createRoute('home', HomeMain)

const allPageRoutes = createRoute('all', AllPage)

const favoritePageRoutes = createRoute('favorite', FavoritePage)

const archivedPageRoutes = createRoute('archived', ArchivedPage)

const trashedPageRoutes = createRoute('trashed', TrashedPage)

const albumsPageRoutes = createRoute('albums', AlbumsPage)

const videosPageRoutes = createRoute('videos', VideosPage)

// ======================================
// Combine All Routes
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
// Create and Export the Router Instance
// ======================================

const router = createRouter({
  history: createWebHistory(),
  routes
})

// Update the browser tab title based on the current route
router.afterEach((to) => {
  const baseName =
    typeof to.meta.baseName === 'string'
      ? to.meta.baseName
      : typeof to.name === 'string'
      ? to.name
      : undefined

  const baseTitleMap: Record<string, string> = {
    home: 'Home',
    all: 'All',
    favorite: 'Favorites',
    archived: 'Archived',
    trashed: 'Trash',
    albums: 'Albums',
    videos: 'Videos',
    album: 'Album',
    tags: 'Tags',
    links: 'Links',
    login: 'Login',
    share: 'Share'
  }

  let baseTitle: string
  if (baseName != null && baseName !== '') {
    baseTitle = baseTitleMap[baseName] ?? baseName
  } else {
    baseTitle = typeof to.name === 'string' ? to.name : 'Urocissa'
  }
  const isView = typeof to.meta.isViewPage === 'boolean' ? to.meta.isViewPage : false
  const isRead = typeof to.meta.isReadPage === 'boolean' ? to.meta.isReadPage : false

  // When on a View page, append the hash (or subhash for read-view) to the title
  let suffix = ''
  if (isView) {
    const maybeHash =
      typeof to.params.subhash === 'string'
        ? to.params.subhash
        : typeof to.params.hash === 'string'
        ? to.params.hash
        : undefined
    if (maybeHash != null && maybeHash !== '') {
      suffix = `View ${maybeHash}`
    } else {
      suffix = 'View'
    }
  } else if (isRead) {
    suffix = 'Reader'
  }

  const pageTitle = suffix ? `${baseTitle} ${suffix}` : baseTitle

  document.title = `${pageTitle} - Urocissa`
})

// On first app load, if user lands directly on a nested page (view/read),
// synthesize the parent entry in history so a simple router.back() returns to it.
void router.isReady().then(async () => {
  const to = router.currentRoute.value

  const meta = to.meta

  // Only act on initial load for nested pages (read/view)
  const isNested = meta.level > 1
  if (isNested) {
    const routeName = typeof to.name === 'string' ? to.name : undefined
    const baseName = typeof meta.baseName === 'string' ? meta.baseName : undefined
    if (routeName === undefined || baseName === undefined) return

    const q = to.query
    const hashParam = typeof to.params.hash === 'string' ? to.params.hash : undefined
    // subhash is not needed for ancestors, only for target which we restore via fullPath

    const chain: RouteLocationRaw[] = []
    // Always build from top-most parent to immediate parent to ensure multi-step back works
    if (routeName === `${baseName}ReadViewPage`) {
      if (hashParam === undefined) return
      chain.push({ name: baseName, query: q })
      chain.push({ name: `${baseName}ViewPage`, params: { hash: hashParam }, query: q })
      chain.push({ name: `${baseName}ReadPage`, params: { hash: hashParam }, query: q })
    } else if (routeName === `${baseName}ReadPage`) {
      if (hashParam === undefined) return
      chain.push({ name: baseName, query: q })
      chain.push({ name: `${baseName}ViewPage`, params: { hash: hashParam }, query: q })
    } else if (routeName === `${baseName}ViewPage`) {
      chain.push({ name: baseName, query: q })
    }

    if (chain.length > 0) {
      const target: RouteLocationRaw = { path: to.fullPath }
      try {
        // Replace current entry with the first ancestor, then push remaining ancestors, then restore target
        const first = chain[0]
        if (first !== undefined) {
          await router.replace(first)
          for (let i = 1; i < chain.length; i++) {
            const step = chain[i]
            if (step !== undefined) {
              await router.push(step)
            }
          }
        }
        await router.push(target)
      } catch {
        // No-op on navigation aborts
      }
    }
  }
})

export default router
