import { notFound } from 'next/navigation'
import { setRequestLocale } from 'next-intl/server'
import { useTranslations } from 'next-intl'
import { Link } from '@/i18n/navigation'
import { db } from '@/lib/db'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { getInitials } from '@/lib/utils'
import { Calendar, Clock, Eye, ArrowLeft, Share2 } from 'lucide-react'

async function getPost(slug: string, locale: string) {
  const post = await db.post.findFirst({
    where: {
      slug,
      locale,
      status: 'PUBLISHED',
    },
    include: {
      author: { select: { id: true, name: true, avatar: true, bio: true } },
      category: { select: { id: true, name: true, slug: true } },
      tags: { include: { tag: true } },
    },
  })

  if (post) {
    await db.post.update({
      where: { id: post.id },
      data: { viewCount: { increment: 1 } },
    })
  }

  return post
}

async function getRelatedPosts(postId: string, categoryId: string | null, locale: string) {
  if (!categoryId) return []
  
  return await db.post.findMany({
    where: {
      id: { not: postId },
      categoryId,
      locale,
      status: 'PUBLISHED',
    },
    select: {
      id: true,
      title: true,
      slug: true,
      excerpt: true,
      coverImage: true,
      publishedAt: true,
    },
    take: 3,
  })
}

interface PostPageProps {
  params: Promise<{ locale: string; slug: string }>
}

export async function generateMetadata({ params }: PostPageProps) {
  const { locale, slug } = await params
  const post = await getPost(slug, locale)
  
  if (!post) return { title: 'Post Not Found' }
  
  return {
    title: post.title,
    description: post.excerpt,
    openGraph: {
      title: post.title,
      description: post.excerpt || '',
      images: post.coverImage ? [post.coverImage] : [],
    },
  }
}

export default async function PostPage({ params }: PostPageProps) {
  const { locale, slug } = await params
  setRequestLocale(locale)

  const post = await getPost(slug, locale)

  if (!post) {
    notFound()
  }

  const relatedPosts = await getRelatedPosts(post.id, post.categoryId, locale)
  const readingTime = Math.ceil((post.content?.length || 0) / 1000)

  return <PostContent post={post} relatedPosts={relatedPosts} readingTime={readingTime} />
}

interface PostContentProps {
  post: NonNullable<Awaited<ReturnType<typeof getPost>>>
  relatedPosts: Awaited<ReturnType<typeof getRelatedPosts>>
  readingTime: number
}

function PostContent({ post, relatedPosts, readingTime }: PostContentProps) {
  const t = useTranslations('blog')

  return (
    <main className="min-h-screen">
      {/* Hero */}
      <section className="border-b bg-muted/30 py-12">
        <div className="container mx-auto max-w-4xl px-4">
          <Button variant="ghost" size="sm" asChild className="mb-6">
            <Link href="/blog">
              <ArrowLeft className="mr-2 h-4 w-4" />
              {t('title')}
            </Link>
          </Button>

          {post.category && (
            <Badge variant="secondary" className="mb-4">
              {post.category.name}
            </Badge>
          )}

          <h1 className="text-3xl font-bold md:text-4xl lg:text-5xl">{post.title}</h1>

          {post.excerpt && (
            <p className="mt-4 text-lg text-muted-foreground">{post.excerpt}</p>
          )}

          <div className="mt-6 flex flex-wrap items-center gap-6">
            <div className="flex items-center gap-3">
              <Avatar className="h-10 w-10">
                <AvatarImage src={post.author.avatar || undefined} />
                <AvatarFallback>{getInitials(post.author.name)}</AvatarFallback>
              </Avatar>
              <div>
                <p className="font-medium">{post.author.name}</p>
                <p className="text-sm text-muted-foreground">{t('post.by').replace(':', '')}</p>
              </div>
            </div>

            <div className="flex items-center gap-4 text-sm text-muted-foreground">
              <span className="flex items-center gap-1">
                <Calendar className="h-4 w-4" />
                {post.publishedAt?.toLocaleDateString()}
              </span>
              <span className="flex items-center gap-1">
                <Clock className="h-4 w-4" />
                {t('post.readTime', { minutes: readingTime })}
              </span>
              <span className="flex items-center gap-1">
                <Eye className="h-4 w-4" />
                {t('post.views', { count: post.viewCount })}
              </span>
            </div>
          </div>
        </div>
      </section>

      {/* Cover Image */}
      {post.coverImage && (
        <section className="border-b">
          <div className="container mx-auto max-w-4xl px-4 py-8">
            <img
              src={post.coverImage}
              alt={post.title}
              className="w-full rounded-lg object-cover"
            />
          </div>
        </section>
      )}

      {/* Content */}
      <section className="py-12">
        <div className="container mx-auto max-w-4xl px-4">
          <article
            className="prose prose-lg dark:prose-invert max-w-none"
            dangerouslySetInnerHTML={{ __html: post.content || '' }}
          />

          {/* Tags */}
          {post.tags.length > 0 && (
            <div className="mt-8 flex flex-wrap gap-2 border-t pt-8">
              {post.tags.map(({ tag }) => (
                <Badge key={tag.id} variant="outline">
                  #{tag.name}
                </Badge>
              ))}
            </div>
          )}

          {/* Share */}
          <div className="mt-8 flex items-center gap-4 border-t pt-8">
            <span className="font-medium">{t('post.share')}:</span>
            <Button variant="outline" size="sm">
              <Share2 className="mr-2 h-4 w-4" />
              {t('post.share')}
            </Button>
          </div>
        </div>
      </section>

      {/* Related Posts */}
      {relatedPosts.length > 0 && (
        <section className="border-t bg-muted/30 py-12">
          <div className="container mx-auto max-w-4xl px-4">
            <h2 className="mb-6 text-2xl font-bold">{t('post.related')}</h2>
            <div className="grid gap-6 md:grid-cols-3">
              {relatedPosts.map((relatedPost) => (
                <Link
                  key={relatedPost.id}
                  href={`/blog/${relatedPost.slug}`}
                  className="group rounded-lg border bg-background p-4 transition-colors hover:bg-accent"
                >
                  {relatedPost.coverImage && (
                    <img
                      src={relatedPost.coverImage}
                      alt={relatedPost.title}
                      className="mb-3 aspect-video w-full rounded object-cover"
                    />
                  )}
                  <h3 className="font-medium group-hover:text-primary">
                    {relatedPost.title}
                  </h3>
                  <p className="mt-1 line-clamp-2 text-sm text-muted-foreground">
                    {relatedPost.excerpt}
                  </p>
                </Link>
              ))}
            </div>
          </div>
        </section>
      )}
    </main>
  )
}
