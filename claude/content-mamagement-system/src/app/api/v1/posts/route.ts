import { NextRequest } from 'next/server'
import { db } from '@/lib/db'
import { getCurrentUser } from '@/lib/auth/jwt'
import { postCreateSchema, paginationSchema } from '@/schemas'
import { responses, handleApiError, validationErrorResponse } from '@/lib/api/response'
import { generateSlug } from '@/lib/utils'

export async function GET(request: NextRequest) {
  try {
    const { searchParams } = new URL(request.url)
    
    const result = paginationSchema.safeParse({
      page: searchParams.get('page'),
      limit: searchParams.get('limit'),
      search: searchParams.get('search'),
      sort: searchParams.get('sort'),
      order: searchParams.get('order'),
    })

    const { page = 1, limit = 10, search, sort = 'createdAt', order = 'desc' } = result.data || {}
    const skip = (page - 1) * limit

    const status = searchParams.get('status')
    const categoryId = searchParams.get('categoryId')
    const locale = searchParams.get('locale')
    const authorId = searchParams.get('authorId')

    const where: Record<string, unknown> = {}
    
    if (search) {
      where.OR = [
        { title: { contains: search, mode: 'insensitive' } },
        { excerpt: { contains: search, mode: 'insensitive' } },
      ]
    }
    if (status) where.status = status
    if (categoryId) where.categoryId = categoryId
    if (locale) where.locale = locale
    if (authorId) where.authorId = authorId

    const [posts, total] = await Promise.all([
      db.post.findMany({
        where,
        include: {
          author: { select: { id: true, name: true, avatar: true } },
          category: { select: { id: true, name: true, slug: true } },
          tags: { include: { tag: true } },
        },
        orderBy: { [sort]: order },
        skip,
        take: limit,
      }),
      db.post.count({ where }),
    ])

    return responses.ok({
      posts,
      pagination: {
        page,
        limit,
        total,
        totalPages: Math.ceil(total / limit),
      },
    })
  } catch (error) {
    return handleApiError(error)
  }
}

export async function POST(request: NextRequest) {
  try {
    const user = await getCurrentUser()
    if (!user) {
      return responses.unauthorized()
    }

    const body = await request.json()
    const result = postCreateSchema.safeParse(body)
    
    if (!result.success) {
      return validationErrorResponse(result.error)
    }

    const { title, content, excerpt, coverImage, categoryId, tags, status, locale, metaTitle, metaDescription } = result.data

    const slug = generateSlug(title)

    const existingPost = await db.post.findFirst({
      where: { slug, locale },
    })

    if (existingPost) {
      return responses.conflict('A post with this title already exists')
    }

    const post = await db.post.create({
      data: {
        title,
        slug,
        content,
        excerpt,
        coverImage,
        categoryId,
        authorId: user.sub,
        status: status || 'DRAFT',
        locale: locale || 'az',
        metaTitle,
        metaDescription,
        publishedAt: status === 'PUBLISHED' ? new Date() : null,
        tags: tags?.length
          ? {
              create: tags.map((tagId: string) => ({
                tag: { connect: { id: tagId } },
              })),
            }
          : undefined,
      },
      include: {
        author: { select: { id: true, name: true, avatar: true } },
        category: { select: { id: true, name: true, slug: true } },
        tags: { include: { tag: true } },
      },
    })

    return responses.created({ post }, 'Post created successfully')
  } catch (error) {
    return handleApiError(error)
  }
}
