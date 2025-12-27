import { NextResponse } from 'next/server'
import { ZodError } from 'zod'

export type ApiResponseData<T = unknown> = {
  success: boolean
  message?: string
  data?: T
  error?: {
    code: string
    message: string
    details?: Record<string, string[]>
  }
}

export function successResponse<T>(data: T, message?: string, status: number = 200) {
  return NextResponse.json({ success: true, message, data }, { status })
}

export function errorResponse(
  message: string,
  code: string = 'ERROR',
  status: number = 400,
  details?: Record<string, string[]>
) {
  return NextResponse.json({ success: false, error: { code, message, details } }, { status })
}

export function validationErrorResponse(error: ZodError) {
  const details: Record<string, string[]> = {}
  error.errors.forEach((err) => {
    const path = err.path.join('.')
    if (!details[path]) details[path] = []
    details[path].push(err.message)
  })
  return errorResponse('Validation failed', 'VALIDATION_ERROR', 400, details)
}

export function handleApiError(error: unknown) {
  console.error('API Error:', error)

  if (error instanceof ZodError) {
    return validationErrorResponse(error)
  }

  if (error instanceof Error) {
    if (error.message.includes('Unique constraint')) {
      return errorResponse('A record with this value already exists', 'DUPLICATE_ERROR', 409)
    }
    if (error.message.includes('not found')) {
      return errorResponse('Resource not found', 'NOT_FOUND', 404)
    }
    if (error.message.includes('Unauthorized')) {
      return errorResponse('Authentication required', 'UNAUTHORIZED', 401)
    }
    if (error.message.includes('Forbidden')) {
      return errorResponse('Permission denied', 'FORBIDDEN', 403)
    }
    return errorResponse(
      process.env.NODE_ENV === 'development' ? error.message : 'An error occurred',
      'INTERNAL_ERROR',
      500
    )
  }

  return errorResponse('An unexpected error occurred', 'INTERNAL_ERROR', 500)
}

export const responses = {
  ok: <T>(data: T, message?: string) => successResponse(data, message, 200),
  created: <T>(data: T, message?: string) => successResponse(data, message, 201),
  noContent: () => new NextResponse(null, { status: 204 }),
  badRequest: (message: string, details?: Record<string, string[]>) => 
    errorResponse(message, 'BAD_REQUEST', 400, details),
  unauthorized: (message: string = 'Authentication required') => 
    errorResponse(message, 'UNAUTHORIZED', 401),
  forbidden: (message: string = 'Permission denied') => 
    errorResponse(message, 'FORBIDDEN', 403),
  notFound: (message: string = 'Resource not found') => 
    errorResponse(message, 'NOT_FOUND', 404),
  conflict: (message: string = 'Resource already exists') => 
    errorResponse(message, 'CONFLICT', 409),
  serverError: (message: string = 'Internal server error') => 
    errorResponse(message, 'INTERNAL_ERROR', 500),
}
