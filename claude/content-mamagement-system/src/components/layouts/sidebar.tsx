'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { useTranslations } from 'next-intl'
import { cn } from '@/lib/utils'
import { useUIStore } from '@/stores/ui-store'
import { useAuth } from '@/hooks/use-auth'
import {
  LayoutDashboard,
  Users,
  Shield,
  FileText,
  FolderTree,
  Tags,
  Image,
  Mail,
  Languages,
  Globe,
  Settings,
  MessageSquare,
  User,
  ChevronLeft,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'

const iconMap: Record<string, React.ElementType> = {
  LayoutDashboard,
  Users,
  Shield,
  FileText,
  FolderTree,
  Tags,
  Image,
  Mail,
  Languages,
  Globe,
  Settings,
  MessageSquare,
  User,
}

interface SidebarProps {
  variant: 'dashboard' | 'admin'
}

export function Sidebar({ variant }: SidebarProps) {
  const pathname = usePathname()
  const t = useTranslations(variant === 'admin' ? 'admin.sidebar' : 'dashboard')
  const { sidebarCollapsed, toggleSidebarCollapsed } = useUIStore()
  const { hasPermission } = useAuth()

  const dashboardItems = [
    { key: 'dashboard', href: '/dashboard', icon: 'LayoutDashboard' },
    { key: 'profile', href: '/dashboard/profile', icon: 'User' },
    { key: 'chat', href: '/dashboard/chat', icon: 'MessageSquare' },
  ]

  const adminItems = [
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
  ]

  const items = variant === 'admin' ? adminItems : dashboardItems

  const filteredItems = items.filter((item) => {
    if (!('permission' in item) || !item.permission) return true
    return hasPermission(item.permission)
  })

  return (
    <aside
      className={cn(
        'relative flex h-screen flex-col border-r bg-sidebar transition-all duration-300',
        sidebarCollapsed ? 'w-16' : 'w-64'
      )}
    >
      <div className="flex h-16 items-center justify-between border-b px-4">
        {!sidebarCollapsed && (
          <Link href="/" className="text-xl font-bold text-sidebar-foreground">
            Qiyas
          </Link>
        )}
        <Button variant="ghost" size="icon" onClick={toggleSidebarCollapsed} className="h-8 w-8">
          <ChevronLeft className={cn('h-4 w-4 transition-transform', sidebarCollapsed && 'rotate-180')} />
        </Button>
      </div>

      <ScrollArea className="flex-1 py-4">
        <nav className="space-y-1 px-2">
          {filteredItems.map((item) => {
            const Icon = iconMap[item.icon] || LayoutDashboard
            const isActive = pathname.includes(item.href)

            return (
              <Link
                key={item.key}
                href={item.href}
                className={cn(
                  'flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors',
                  isActive
                    ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                    : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
                )}
              >
                <Icon className="h-5 w-5 shrink-0" />
                {!sidebarCollapsed && <span>{t(item.key)}</span>}
              </Link>
            )
          })}
        </nav>
      </ScrollArea>
    </aside>
  )
}
