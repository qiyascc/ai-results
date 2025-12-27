import { setRequestLocale } from 'next-intl/server'
import { useTranslations } from 'next-intl'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Users, FileText, Eye, MessageSquare } from 'lucide-react'

interface DashboardPageProps {
  params: Promise<{ locale: string }>
}

export default async function DashboardPage({ params }: DashboardPageProps) {
  const { locale } = await params
  setRequestLocale(locale)

  return <DashboardContent />
}

function DashboardContent() {
  const t = useTranslations('dashboard')

  const stats = [
    { key: 'totalViews', icon: Eye, value: '12,543', change: '+12%' },
    { key: 'totalPosts', icon: FileText, value: '48', change: '+3' },
    { key: 'totalComments', icon: MessageSquare, value: '234', change: '+18' },
    { key: 'newUsers', icon: Users, value: '156', change: '+24%' },
  ]

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">{t('title')}</h1>
        <p className="text-muted-foreground">{t('welcome')}</p>
      </div>

      {/* Stats Cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {stats.map((stat) => (
          <Card key={stat.key}>
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                {t(`stats.${stat.key}`)}
              </CardTitle>
              <stat.icon className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{stat.value}</div>
              <p className="text-xs text-muted-foreground">{stat.change} from last month</p>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Quick Actions */}
      <Card>
        <CardHeader>
          <CardTitle>{t('quickActions.title')}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-4">
            {['newPost', 'uploadMedia', 'viewSite', 'viewAdmin'].map((action) => (
              <button
                key={action}
                className="flex items-center justify-center gap-2 rounded-lg border p-4 hover:bg-accent"
              >
                {t(`quickActions.${action}`)}
              </button>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* Recent Activity */}
      <Card>
        <CardHeader>
          <CardTitle>{t('recentActivity')}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {[1, 2, 3, 4, 5].map((i) => (
              <div key={i} className="flex items-center gap-4 rounded-lg border p-3">
                <div className="h-10 w-10 rounded-full bg-muted" />
                <div className="flex-1">
                  <p className="text-sm font-medium">Activity item {i}</p>
                  <p className="text-xs text-muted-foreground">2 hours ago</p>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
