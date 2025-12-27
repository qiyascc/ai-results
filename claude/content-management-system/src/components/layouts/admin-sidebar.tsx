'use client'

import { useTranslations } from 'next-intl'
import { usePathname } from 'next/navigation'
import { Link } from '@/i18n/navigation'
import { cn } from '@/lib/utils'
import {
  LayoutDashboard,
  Users,
  Shield,
  FileText,
  FolderOpen,
  Tags,
  Image,
  MessageSquare,
  Languages,
  Globe,
  Settings,
  BarChart3,
  ClipboardList,
} from 'lucide-react'
import { ScrollArea } from '@/components/ui/scroll-area'

const menuItems = [
  { key: 'dashboard', href: '/admin', icon: LayoutDashboard },
  { key: 'users', href: '/admin/users', icon: Users },
  { key: 'roles', href: '/admin/roles', icon: Shield },
  { key: 'posts', href: '/admin/posts', icon: FileText },
  { key: 'categories', href: '/admin/categories', icon: FolderOpen },
  { key: 'tags', href: '/admin/tags', icon: Tags },
  { key: 'media', href: '/admin/media', icon: Image },
  { key: 'contacts', href: '/admin/contacts', icon: MessageSquare },
  { key: 'languages', href: '/admin/languages', icon: Languages },
  { key: 'translations', href: '/admin/translations', icon: Globe },
  { key: 'analytics', href: '/admin/analytics', icon: BarChart3 },
  { key: 'audit', href: '/admin/audit', icon: ClipboardList },
  { key: 'settings', href: '/admin/settings', icon: Settings },
]

export function AdminSidebar() {
  const t = useTranslations('admin.sidebar')
  const pathname = usePathname()

  const isActive = (href: string) => {
    const pathWithoutLocale = pathname.replace(/^\/(az|en|tr)/, '')
    if (href === '/admin') {
      return pathWithoutLocale === '/admin'
    }
    return pathWithoutLocale.startsWith(href)
  }

  return (
    <aside className="hidden w-64 border-r bg-sidebar lg:block">
      <div className="flex h-16 items-center border-b px-6">
        <Link href="/admin" className="flex items-center gap-2 font-bold text-xl">
          <span className="text-primary">Qiyas</span>
          <span className="text-muted-foreground text-sm">Admin</span>
        </Link>
      </div>

      <ScrollArea className="h-[calc(100vh-4rem)]">
        <nav className="space-y-1 p-4">
          {menuItems.map((item) => (
            <Link
              key={item.key}
              href={item.href}
              className={cn(
                'flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors',
                isActive(item.href)
                  ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                  : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
              )}
            >
              <item.icon className="h-4 w-4" />
              {t(item.key)}
            </Link>
          ))}
        </nav>
      </ScrollArea>
    </aside>
  )
}
