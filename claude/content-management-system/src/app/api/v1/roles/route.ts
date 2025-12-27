import { NextRequest } from 'next/server'
import { db } from '@/lib/db'
import { getCurrentUser } from '@/lib/auth/jwt'
import { paginationSchema } from '@/schemas'
import { responses, handleApiError, validationErrorResponse } from '@/lib/api/response'
import { z } from 'zod'

const roleSchema = z.object({
  name: z.string().min(2).max(50),
  slug: z.string().min(2).max(50).regex(/^[a-z0-9-]+$/),
  description: z.string().optional(),
  permissions: z.array(z.string()).optional(),
})

export async function GET(request: NextRequest) {
  try {
    const user = await getCurrentUser()
    if (!user) {
      return responses.unauthorized()
    }

    if (!user.permissions.includes('roles:read') && user.role !== 'super-admin') {
      return responses.forbidden('You do not have permission to view roles')
    }

    const { searchParams } = new URL(request.url)
    
    const result = paginationSchema.safeParse({
      page: searchParams.get('page'),
      limit: searchParams.get('limit'),
      search: searchParams.get('search'),
    })

    const { page = 1, limit = 50, search } = result.data || {}
    const skip = (page - 1) * limit

    const where: Record<string, unknown> = {}
    
    if (search) {
      where.OR = [
        { name: { contains: search, mode: 'insensitive' } },
        { slug: { contains: search, mode: 'insensitive' } },
      ]
    }

    const [roles, total] = await Promise.all([
      db.role.findMany({
        where,
        include: {
          permissions: {
            include: { permission: true },
          },
          _count: { select: { users: true } },
        },
        orderBy: { name: 'asc' },
        skip,
        take: limit,
      }),
      db.role.count({ where }),
    ])

    return responses.ok({
      roles: roles.map((role) => ({
        ...role,
        permissions: role.permissions.map((rp) => rp.permission),
        userCount: role._count.users,
      })),
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

    if (!user.permissions.includes('roles:create') && user.role !== 'super-admin') {
      return responses.forbidden('You do not have permission to create roles')
    }

    const body = await request.json()
    const result = roleSchema.safeParse(body)
    
    if (!result.success) {
      return validationErrorResponse(result.error)
    }

    const { name, slug, description, permissions } = result.data

    const existingRole = await db.role.findUnique({ where: { slug } })
    if (existingRole) {
      return responses.conflict('A role with this slug already exists')
    }

    const role = await db.role.create({
      data: {
        name,
        slug,
        description,
        permissions: permissions?.length
          ? {
              create: permissions.map((permissionId) => ({
                permission: { connect: { id: permissionId } },
              })),
            }
          : undefined,
      },
      include: {
        permissions: { include: { permission: true } },
      },
    })

    return responses.created(
      {
        role: {
          ...role,
          permissions: role.permissions.map((rp) => rp.permission),
        },
      },
      'Role created successfully'
    )
  } catch (error) {
    return handleApiError(error)
  }
}
