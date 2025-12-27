import { createSharedPathnamesNavigation } from 'next-intl/navigation'
import { locales } from './config'

export const localePrefix = 'always'

export const { Link, redirect, usePathname, useRouter } = createSharedPathnamesNavigation({
  locales,
  localePrefix,
})

export const publicNavItems = [
  { key: 'home', href: '/' },
  { key: 'blog', href: '/blog' },
  { key: 'about', href: '/about' },
  { key: 'contact', href: '/contact' },
] as const

export const dashboardNavItems = [
  { key: 'dashboard', href: '/dashboard', icon: 'LayoutDashboard' },
  { key: 'profile', href: '/dashboard/profile', icon: 'User' },
  { key: 'chat', href: '/dashboard/chat', icon: 'MessageSquare' },
] as const

export const adminNavItems = [
  { key: 'dashboard', href: '/admin', icon: 'LayoutDashboard', permission: null },
  { key: 'users', href: '/admin/users', icon: 'Users', permission: 'users:read' },
  { key: 'roles', href: '/admin/roles', icon: 'Shield', permission: 'roles:read' },
  { key: 'posts', href: '/admin/posts', icon: 'FileText', permission: 'posts:read' },
  { key: 'categories', href: '/admin/categories', icon: 'FolderTree', permission: 'categories:read' },
  { key: 'tags', href: '/admin/tags', icon: 'Tags', permission: 'tags:read' },
  { key: 'media', href: '/admin/media', icon: 'Image', permission: 'media:read' },
  { key: 'contacts', href: '/admin/contacts', icon: 'Mail', permission: 'contacts:read' },
  { key: 'languages', href: '/admin/languages', icon: 'Languages', permission: 'languages:manage' },
  { key: 'translations', href: '/admin/translations', icon: 'Globe', permission: 'translations:manage' },
  { key: 'settings', href: '/admin/settings', icon: 'Settings', permission: 'settings:read' },
] as const
