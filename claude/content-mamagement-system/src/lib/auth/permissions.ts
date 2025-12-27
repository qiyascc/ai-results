import { db } from '@/lib/db'
import { getCurrentUser, type TokenPayload } from './jwt'

export const PERMISSIONS = {
  USERS_CREATE: 'users:create',
  USERS_READ: 'users:read',
  USERS_UPDATE: 'users:update',
  USERS_DELETE: 'users:delete',
  ROLES_CREATE: 'roles:create',
  ROLES_READ: 'roles:read',
  ROLES_UPDATE: 'roles:update',
  ROLES_DELETE: 'roles:delete',
  POSTS_CREATE: 'posts:create',
  POSTS_READ: 'posts:read',
  POSTS_UPDATE: 'posts:update',
  POSTS_DELETE: 'posts:delete',
  POSTS_PUBLISH: 'posts:publish',
  POSTS_UPDATE_OWN: 'posts:update:own',
  POSTS_DELETE_OWN: 'posts:delete:own',
  CATEGORIES_CREATE: 'categories:create',
  CATEGORIES_READ: 'categories:read',
  CATEGORIES_UPDATE: 'categories:update',
  CATEGORIES_DELETE: 'categories:delete',
  TAGS_CREATE: 'tags:create',
  TAGS_READ: 'tags:read',
  TAGS_UPDATE: 'tags:update',
  TAGS_DELETE: 'tags:delete',
  MEDIA_CREATE: 'media:create',
  MEDIA_READ: 'media:read',
  MEDIA_UPDATE: 'media:update',
  MEDIA_DELETE: 'media:delete',
  LANGUAGES_MANAGE: 'languages:manage',
  TRANSLATIONS_MANAGE: 'translations:manage',
  SETTINGS_READ: 'settings:read',
  SETTINGS_UPDATE: 'settings:update',
  CONTACTS_READ: 'contacts:read',
  CONTACTS_UPDATE: 'contacts:update',
  CONTACTS_DELETE: 'contacts:delete',
  AUDIT_READ: 'audit:read',
  CHAT_ACCESS: 'chat:access',
  ANALYTICS_READ: 'analytics:read',
} as const

export type Permission = (typeof PERMISSIONS)[keyof typeof PERMISSIONS]

export function hasPermission(userPermissions: string[], requiredPermission: Permission): boolean {
  return userPermissions.includes(requiredPermission)
}

export function hasAnyPermission(userPermissions: string[], requiredPermissions: Permission[]): boolean {
  return requiredPermissions.some((p) => userPermissions.includes(p))
}

export function hasAllPermissions(userPermissions: string[], requiredPermissions: Permission[]): boolean {
  return requiredPermissions.every((p) => userPermissions.includes(p))
}

export async function getUserPermissions(userId: string): Promise<string[]> {
  const user = await db.user.findUnique({
    where: { id: userId },
    include: {
      role: {
        include: {
          permissions: {
            include: { permission: true },
          },
        },
      },
    },
  })
  
  if (!user) return []
  return user.role.permissions.map((rp) => rp.permission.name)
}

export async function checkPermission(
  requiredPermission: Permission
): Promise<{ allowed: boolean; user: TokenPayload | null }> {
  const user = await getCurrentUser()
  if (!user) return { allowed: false, user: null }
  const allowed = hasPermission(user.permissions, requiredPermission)
  return { allowed, user }
}
