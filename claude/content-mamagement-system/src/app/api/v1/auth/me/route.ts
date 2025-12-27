import { db } from '@/lib/db'
import { getCurrentUser } from '@/lib/auth/jwt'
import { responses, handleApiError } from '@/lib/api/response'

export async function GET() {
  try {
    const payload = await getCurrentUser()
    
    if (!payload) {
      return responses.unauthorized('Not authenticated')
    }

    const user = await db.user.findUnique({
      where: { id: payload.sub },
      include: {
        role: {
          include: {
            permissions: { include: { permission: true } },
          },
        },
      },
    })

    if (!user) {
      return responses.notFound('User not found')
    }

    const permissions = user.role.permissions.map((rp) => rp.permission.name)

    return responses.ok({
      user: {
        id: user.id,
        email: user.email,
        name: user.name,
        avatar: user.avatar,
        role: user.role.slug,
        permissions,
        locale: user.locale,
      },
    })
  } catch (error) {
    return handleApiError(error)
  }
}
