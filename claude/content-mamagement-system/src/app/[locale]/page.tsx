import { useTranslations } from 'next-intl'
import Link from 'next/link'
import { Button } from '@/components/ui/button'

export default function HomePage() {
  const t = useTranslations('common')

  return (
    <main className="min-h-screen">
      {/* Hero Section */}
      <section className="relative flex min-h-[80vh] flex-col items-center justify-center bg-gradient-to-b from-background to-muted px-4">
        <div className="container mx-auto text-center">
          <h1 className="mb-6 text-4xl font-bold tracking-tight sm:text-5xl md:text-6xl">
            Qiyas <span className="text-primary">CMS</span>
          </h1>
          <p className="mx-auto mb-8 max-w-2xl text-lg text-muted-foreground sm:text-xl">
            {t('metadata.description')}
          </p>
          <div className="flex flex-col gap-4 sm:flex-row sm:justify-center">
            <Button asChild size="lg">
              <Link href="/login">{t('navigation.login')}</Link>
            </Button>
            <Button asChild variant="outline" size="lg">
              <Link href="/blog">{t('navigation.blog')}</Link>
            </Button>
          </div>
        </div>
      </section>

      {/* Features Section */}
      <section className="py-20">
        <div className="container mx-auto px-4">
          <h2 className="mb-12 text-center text-3xl font-bold">Features</h2>
          <div className="grid gap-8 md:grid-cols-3">
            {[
              { icon: '🌍', title: 'Multi-Language', desc: 'AZ, EN, TR support' },
              { icon: '🔐', title: 'RBAC', desc: 'Role-based access control' },
              { icon: '💬', title: 'AI Chat', desc: 'AI-powered conversations' },
              { icon: '📝', title: 'Rich Editor', desc: 'TipTap editor' },
              { icon: '📁', title: 'Media Library', desc: 'Image management' },
              { icon: '📊', title: 'Analytics', desc: 'Built-in tracking' },
            ].map((feature, i) => (
              <div key={i} className="rounded-lg border bg-card p-6 text-center shadow-sm">
                <div className="mb-4 text-4xl">{feature.icon}</div>
                <h3 className="mb-2 text-xl font-semibold">{feature.title}</h3>
                <p className="text-muted-foreground">{feature.desc}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t py-8">
        <div className="container mx-auto px-4 text-center text-sm text-muted-foreground">
          {t('footer.copyright', { year: new Date().getFullYear() })}
        </div>
      </footer>
    </main>
  )
}
