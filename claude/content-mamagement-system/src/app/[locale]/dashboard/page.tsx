'use client'

import { useTranslations } from 'next-intl'
import { useAuth } from '@/hooks/use-auth'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { LayoutDashboard, FileText, Users, Image } from 'lucide-react'

export default function DashboardPage() {
  const t = useTranslations('dashboard')
  const { user, isLoading } = useAuth()

  if (isLoading) {
    return (
      <div className="container mx-auto p-6">
        <Skeleton className="mb-6 h-10 w-64" />
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          {[1, 2, 3, 4].map((i) => (
            <Skeleton key={i} className="h-32" />
          ))}
        </div>
      </div>
    )
  }

  const stats = [
    { title: t('stats.totalViews'), value: '12,345', icon: LayoutDashboard, color: 'text-blue-500' },
    { title: t('stats.totalPosts'), value: '48', icon: FileText, color: 'text-green-500' },
    { title: t('stats.totalComments'), value: '256', icon: Users, color: 'text-purple-500' },
    { title: t('stats.newUsers'), value: '18', icon: Image, color: 'text-orange-500' },
  ]

  return (
    <div className="container mx-auto p-6">
      <div className="mb-6">
        <h1 className="text-3xl font-bold">{t('title')}</h1>
        <p className="text-muted-foreground">{t('welcome')} {user?.name}</p>
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
            <CardTitle>{t('quickActions.title')}</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-wrap gap-2">
            <a href="/admin/posts/new" className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90">
              {t('quickActions.newPost')}
            </a>
            <a href="/admin/media" className="rounded-md bg-secondary px-4 py-2 text-sm text-secondary-foreground hover:bg-secondary/80">
              {t('quickActions.uploadMedia')}
            </a>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t('recentActivity')}</CardTitle>
            <CardDescription>Son aktiviteler burada görünecek</CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground">Henüz aktivite yok</p>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
