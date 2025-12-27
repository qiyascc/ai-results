'use client'

import { useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { useAuthStore } from '@/stores/auth-store'

export function useAuth() {
  const { user, isAuthenticated, isLoading, login, logout, hasPermission, hasAnyPermission } =
    useAuthStore()

  return {
    user,
    isAuthenticated,
    isLoading,
    login,
    logout,
    hasPermission,
    hasAnyPermission,
  }
}

export function useRequireAuth(redirectTo: string = '/login') {
  const router = useRouter()
  const { isAuthenticated, isLoading } = useAuthStore()

  useEffect(() => {
    if (!isLoading && !isAuthenticated) {
      router.push(redirectTo)
    }
  }, [isAuthenticated, isLoading, redirectTo, router])

  return { isAuthenticated, isLoading }
}

export function useRequirePermission(permission: string, redirectTo: string = '/') {
  const router = useRouter()
  const { isAuthenticated, isLoading, hasPermission } = useAuthStore()

  useEffect(() => {
    if (!isLoading) {
      if (!isAuthenticated) {
        router.push('/login')
      } else if (!hasPermission(permission)) {
        router.push(redirectTo)
      }
    }
  }, [isAuthenticated, isLoading, hasPermission, permission, redirectTo, router])

  return { isAuthenticated, isLoading, hasPermission: hasPermission(permission) }
}
