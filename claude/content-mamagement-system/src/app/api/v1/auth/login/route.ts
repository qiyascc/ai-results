import { NextRequest } from 'next/server'
import { db } from '@/lib/db'
import { verifyPassword } from '@/lib/auth/password'
import { generateAccessToken, generateRefreshToken, setAuthCookies } from '@/lib/auth/jwt'
import { loginSchema } from '@/schemas'
import { responses, handleApiError, validationErrorResponse } from '@/lib/api/response'

export async function POST(request: NextRequest) {
  try {
    const body = await request.json()
    
    const result = loginSchema.safeParse(body)
    if (!result.success) {
      return validationErrorResponse(result.error)
    }

    const { email, password } = result.data

    const user = await db.user.findUnique({
      where: { email: email.toLowerCase() },
      include: {
        role: {
          include: {
            permissions: {
              include: { permission: true },
            },
          },
        },
      },
    })

    if (!user || !user.password) {
      return responses.unauthorized('Invalid email or password')
    }

    if (user.status !== 'ACTIVE') {
      return responses.forbidden('Your account is not active')
    }

    const isValidPassword = await verifyPassword(password, user.password)
    if (!isValidPassword) {
      return responses.unauthorized('Invalid email or password')
    }

    const permissions = user.role.permissions.map((rp) => rp.permission.name)

    const tokenPayload = {
      sub: user.id,
      email: user.email,
      role: user.role.slug,
      permissions,
    }

    const accessToken = await generateAccessToken(tokenPayload)
    const refreshToken = await generateRefreshToken(user.id)

    await db.session.create({
      data: {
        userId: user.id,
        token: accessToken,
        refreshToken,
        userAgent: request.headers.get('user-agent') || undefined,
        ipAddress: request.headers.get('x-forwarded-for') || undefined,
        expiresAt: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000),
      },
    })

    await db.user.update({
      where: { id: user.id },
      data: {
        lastLoginAt: new Date(),
        lastLoginIp: request.headers.get('x-forwarded-for') || undefined,
      },
    })

    await setAuthCookies(accessToken, refreshToken)

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
      accessToken,
    }, 'Login successful')

  } catch (error) {
    return handleApiError(error)
  }
}
