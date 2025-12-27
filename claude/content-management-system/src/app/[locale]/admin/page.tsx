import { setRequestLocale } from 'next-intl/server'
import { useTranslations } from 'next-intl'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Users, FileText, FolderOpen, Image, MessageSquare, Eye } from 'lucide-react'

interface AdminPageProps {
  params: Promise<{ locale: string }>
}

export default async function AdminPage({ params }: AdminPageProps) {
  const { locale } = await params
  setRequestLocale(locale)

  return <AdminDashboardContent />
}

function AdminDashboardContent() {
  const t = useTranslations('admin.dashboard')

  const stats = [
    { key: 'users', icon: Users, value: '1,234', color: 'text-blue-500' },
    { key: 'posts', icon: FileText, value: '567', color: 'text-green-500' },
    { key: 'categories', icon: FolderOpen, value: '24', color: 'text-purple-500' },
    { key: 'media', icon: Image, value: '1,892', color: 'text-orange-500' },
    { key: 'views', icon: Eye, value: '45,678', color: 'text-cyan-500' },
    { key: 'contacts', icon: MessageSquare, value: '89', color: 'text-pink-500' },
  ]

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold">{t('title')}</h1>
        <p className="text-muted-foreground">{t('overview')}</p>
      </div>

      {/* Stats Grid */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6">
        {stats.map((stat) => (
          <Card key={stat.key}>
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                {t(`stats.${stat.key}`)}
              </CardTitle>
              <stat.icon className={`h-4 w-4 ${stat.color}`} />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{stat.value}</div>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Recent Data */}
      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
        {/* Recent Posts */}
        <Card>
          <CardHeader>
            <CardTitle>{t('recentPosts')}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {[1, 2, 3, 4, 5].map((i) => (
                <div key={i} className="flex items-center gap-3 rounded border p-2">
                  <div className="h-10 w-10 rounded bg-muted" />
                  <div className="flex-1 truncate">
                    <p className="text-sm font-medium truncate">Post title {i}</p>
                    <p className="text-xs text-muted-foreground">2 hours ago</p>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        {/* Recent Users */}
        <Card>
          <CardHeader>
            <CardTitle>{t('recentUsers')}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {[1, 2, 3, 4, 5].map((i) => (
                <div key={i} className="flex items-center gap-3 rounded border p-2">
                  <div className="h-10 w-10 rounded-full bg-muted" />
                  <div className="flex-1 truncate">
                    <p className="text-sm font-medium">User {i}</p>
                    <p className="text-xs text-muted-foreground">user{i}@example.com</p>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        {/* Recent Contacts */}
        <Card>
          <CardHeader>
            <CardTitle>{t('recentContacts')}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {[1, 2, 3, 4, 5].map((i) => (
                <div key={i} className="flex items-center gap-3 rounded border p-2">
                  <MessageSquare className="h-8 w-8 text-muted-foreground" />
                  <div className="flex-1 truncate">
                    <p className="text-sm font-medium">Contact {i}</p>
                    <p className="text-xs text-muted-foreground truncate">Message preview...</p>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
