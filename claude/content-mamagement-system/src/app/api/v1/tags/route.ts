import { NextRequest } from 'next/server'
import { db } from '@/lib/db'
import { getCurrentUser } from '@/lib/auth/jwt'
import { tagSchema, paginationSchema } from '@/schemas'
import { responses, handleApiError, validationErrorResponse } from '@/lib/api/response'
import { generateSlug } from '@/lib/utils'

export async function GET(request: NextRequest) {
  try {
    const { searchParams } = new URL(request.url)
    
    const result = paginationSchema.safeParse({
      page: searchParams.get('page'),
      limit: searchParams.get('limit'),
      search: searchParams.get('search'),
    })

    const { page = 1, limit = 50, search } = result.data || {}
    const skip = (page - 1) * limit

    const locale = searchParams.get('locale') || 'az'

    const where: Record<string, unknown> = { locale }
    
    if (search) {
      where.OR = [
        { name: { contains: search, mode: 'insensitive' } },
        { slug: { contains: search, mode: 'insensitive' } },
      ]
    }

    const [tags, total] = await Promise.all([
      db.tag.findMany({
        where,
        include: {
          _count: { select: { posts: true } },
        },
        orderBy: { name: 'asc' },
        skip,
        take: limit,
      }),
      db.tag.count({ where }),
    ])

    return responses.ok({
      tags,
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

    if (!user.permissions.includes('tags:create') && user.role !== 'super-admin') {
      return responses.forbidden('You do not have permission to create tags')
    }

    const body = await request.json()
    const result = tagSchema.safeParse(body)
    
    if (!result.success) {
      return validationErrorResponse(result.error)
    }

    const { name, locale } = result.data

    const slug = generateSlug(name)

    const existingTag = await db.tag.findFirst({
      where: { slug, locale },
    })

    if (existingTag) {
      return responses.conflict('A tag with this name already exists')
    }

    const tag = await db.tag.create({
      data: {
        name,
        slug,
        locale: locale || 'az',
      },
    })

    return responses.created({ tag }, 'Tag created successfully')
  } catch (error) {
    return handleApiError(error)
  }
}
