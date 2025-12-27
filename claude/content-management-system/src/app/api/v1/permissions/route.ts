import { NextRequest } from 'next/server'
import { db } from '@/lib/db'
import { getCurrentUser } from '@/lib/auth/jwt'
import { responses, handleApiError } from '@/lib/api/response'

export async function GET(request: NextRequest) {
  try {
    const user = await getCurrentUser()
    if (!user) {
      return responses.unauthorized()
    }

    if (!user.permissions.includes('roles:read') && user.role !== 'super-admin') {
      return responses.forbidden('You do not have permission to view permissions')
    }

    const permissions = await db.permission.findMany({
      orderBy: { name: 'asc' },
    })

    // Group permissions by category
    const grouped = permissions.reduce((acc, perm) => {
      const [category] = perm.name.split(':')
      if (!acc[category]) acc[category] = []
      acc[category].push(perm)
      return acc
    }, {} as Record<string, typeof permissions>)

    return responses.ok({ permissions, grouped })
  } catch (error) {
    return handleApiError(error)
  }
}
