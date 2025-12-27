'use client'

import { useTranslations } from 'next-intl'
import { usePathname } from 'next/navigation'
import { Link } from '@/i18n/navigation'
import { cn } from '@/lib/utils'
import { LayoutDashboard, MessageSquare, User, Settings } from 'lucide-react'
import { ScrollArea } from '@/components/ui/scroll-area'

const menuItems = [
  { key: 'dashboard', href: '/dashboard', icon: LayoutDashboard },
  { key: 'chat', href: '/dashboard/chat', icon: MessageSquare },
  { key: 'profile', href: '/dashboard/profile', icon: User },
  { key: 'settings', href: '/dashboard/settings', icon: Settings },
]

export function Sidebar() {
  const t = useTranslations('dashboard.sidebar')
  const pathname = usePathname()

  const isActive = (href: string) => {
    const pathWithoutLocale = pathname.replace(/^\/(az|en|tr)/, '')
    if (href === '/dashboard') {
      return pathWithoutLocale === '/dashboard'
    }
    return pathWithoutLocale.startsWith(href)
  }

  return (
    <aside className="hidden w-64 border-r bg-sidebar lg:block">
      <div className="flex h-16 items-center border-b px-6">
        <Link href="/" className="flex items-center gap-2 font-bold text-xl">
          <span className="text-primary">Qiyas</span>
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
