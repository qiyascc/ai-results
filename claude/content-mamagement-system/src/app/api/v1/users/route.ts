import { NextRequest } from 'next/server'
import { db } from '@/lib/db'
import { getCurrentUser } from '@/lib/auth/jwt'
import { hashPassword } from '@/lib/auth/password'
import { registerSchema, paginationSchema } from '@/schemas'
import { responses, handleApiError, validationErrorResponse } from '@/lib/api/response'

export async function GET(request: NextRequest) {
  try {
    const user = await getCurrentUser()
    if (!user) {
      return responses.unauthorized()
    }

    // Check permission
    if (!user.permissions.includes('users:read') && user.role !== 'super-admin') {
      return responses.forbidden('You do not have permission to view users')
    }

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
    const roleId = searchParams.get('roleId')

    const where: Record<string, unknown> = {}
    
    if (search) {
      where.OR = [
        { name: { contains: search, mode: 'insensitive' } },
        { email: { contains: search, mode: 'insensitive' } },
      ]
    }
    if (status) where.status = status
    if (roleId) where.roleId = roleId

    const [users, total] = await Promise.all([
      db.user.findMany({
        where,
        select: {
          id: true,
          email: true,
          name: true,
          avatar: true,
          status: true,
          locale: true,
          createdAt: true,
          lastLoginAt: true,
          role: { select: { id: true, name: true, slug: true } },
        },
        orderBy: { [sort]: order },
        skip,
        take: limit,
      }),
      db.user.count({ where }),
    ])

    return responses.ok({
      users,
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
    const currentUser = await getCurrentUser()
    if (!currentUser) {
      return responses.unauthorized()
    }

    // Check permission
    if (!currentUser.permissions.includes('users:create') && currentUser.role !== 'super-admin') {
      return responses.forbidden('You do not have permission to create users')
    }

    const body = await request.json()
    const result = registerSchema.safeParse(body)
    
    if (!result.success) {
      return validationErrorResponse(result.error)
    }

    const { name, email, password } = result.data
    const roleId = body.roleId

    // Check if email already exists
    const existingUser = await db.user.findUnique({ where: { email: email.toLowerCase() } })
    if (existingUser) {
      return responses.conflict('A user with this email already exists')
    }

    // Get default role if not provided
    let userRoleId = roleId
    if (!userRoleId) {
      const defaultRole = await db.role.findFirst({ where: { slug: 'user' } })
      userRoleId = defaultRole?.id
    }

    if (!userRoleId) {
      return responses.badRequest('Role is required')
    }

    const hashedPassword = await hashPassword(password)

    const user = await db.user.create({
      data: {
        name,
        email: email.toLowerCase(),
        password: hashedPassword,
        roleId: userRoleId,
        status: 'ACTIVE',
      },
      select: {
        id: true,
        email: true,
        name: true,
        status: true,
        createdAt: true,
        role: { select: { id: true, name: true, slug: true } },
      },
    })

    return responses.created({ user }, 'User created successfully')
  } catch (error) {
    return handleApiError(error)
  }
}
