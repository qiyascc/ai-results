import { NextRequest } from 'next/server'
import { db } from '@/lib/db'
import { getCurrentUser } from '@/lib/auth/jwt'
import { categorySchema, paginationSchema } from '@/schemas'
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
    const parentId = searchParams.get('parentId')

    const where: Record<string, unknown> = { locale }
    
    if (search) {
      where.OR = [
        { name: { contains: search, mode: 'insensitive' } },
        { slug: { contains: search, mode: 'insensitive' } },
      ]
    }
    
    if (parentId === 'null') {
      where.parentId = null
    } else if (parentId) {
      where.parentId = parentId
    }

    const [categories, total] = await Promise.all([
      db.category.findMany({
        where,
        include: {
          parent: { select: { id: true, name: true, slug: true } },
          children: { select: { id: true, name: true, slug: true } },
          _count: { select: { posts: true } },
        },
        orderBy: { order: 'asc' },
        skip,
        take: limit,
      }),
      db.category.count({ where }),
    ])

    return responses.ok({
      categories,
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

    if (!user.permissions.includes('categories:create') && user.role !== 'super-admin') {
      return responses.forbidden('You do not have permission to create categories')
    }

    const body = await request.json()
    const result = categorySchema.safeParse(body)
    
    if (!result.success) {
      return validationErrorResponse(result.error)
    }

    const { name, description, parentId, locale, metaTitle, metaDescription } = result.data

    const slug = generateSlug(name)

    const existingCategory = await db.category.findFirst({
      where: { slug, locale },
    })

    if (existingCategory) {
      return responses.conflict('A category with this name already exists')
    }

    // Get max order
    const maxOrder = await db.category.aggregate({
      where: { locale, parentId: parentId || null },
      _max: { order: true },
    })

    const category = await db.category.create({
      data: {
        name,
        slug,
        description,
        parentId,
        locale: locale || 'az',
        metaTitle,
        metaDescription,
        order: (maxOrder._max.order || 0) + 1,
      },
      include: {
        parent: { select: { id: true, name: true, slug: true } },
        children: { select: { id: true, name: true, slug: true } },
      },
    })

    return responses.created({ category }, 'Category created successfully')
  } catch (error) {
    return handleApiError(error)
  }
}
