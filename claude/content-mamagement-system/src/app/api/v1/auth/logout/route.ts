import { NextRequest } from 'next/server'
import { db } from '@/lib/db'
import { getTokenFromCookies, clearAuthCookies, verifyToken } from '@/lib/auth/jwt'
import { responses, handleApiError } from '@/lib/api/response'

export async function POST(request: NextRequest) {
  try {
    const token = await getTokenFromCookies()
    
    if (token) {
      const payload = await verifyToken(token)
      if (payload) {
        await db.session.deleteMany({ where: { userId: payload.sub } })
      }
    }

    await clearAuthCookies()

    return responses.ok({ success: true }, 'Logged out successfully')
  } catch (error) {
    return handleApiError(error)
  }
}
