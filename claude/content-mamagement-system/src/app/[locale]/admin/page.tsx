'use client'

import { useTranslations } from 'next-intl'
import { useAuth } from '@/hooks/use-auth'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Users, FileText, FolderTree, Image } from 'lucide-react'

export default function AdminDashboardPage() {
  const t = useTranslations('admin.dashboard')
  const { user } = useAuth()

  const stats = [
    { title: t('stats.users'), value: '125', icon: Users, color: 'text-blue-500' },
    { title: t('stats.posts'), value: '48', icon: FileText, color: 'text-green-500' },
    { title: t('stats.categories'), value: '12', icon: FolderTree, color: 'text-purple-500' },
    { title: t('stats.media'), value: '256', icon: Image, color: 'text-orange-500' },
  ]

  return (
    <div className="container mx-auto p-6">
      <div className="mb-6">
        <h1 className="text-3xl font-bold">{t('title')}</h1>
        <p className="text-muted-foreground">
          {useTranslations('admin')('welcome', { name: user?.name || 'Admin' })}
        </p>
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {stats.map((stat, i) => (
          <Card key={i}>
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium">{stat.title}</CardTitle>
              <stat.icon className={`h-4 w-4 ${stat.color}`} />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{stat.value}</div>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="mt-8 grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>{t('recentPosts')}</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground">Son yazılar burada listelenecek</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t('recentUsers')}</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground">Son kayıtlar burada listelenecek</p>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
