import createMiddleware from 'next-intl/middleware'
import { NextRequest, NextResponse } from 'next/server'
import { routing } from '@/i18n/routing'

const intlMiddleware = createMiddleware(routing)

// Routes that require authentication
const protectedRoutes = ['/dashboard', '/admin', '/chat', '/profile']

// Routes that should redirect to dashboard if authenticated
const authRoutes = ['/login', '/register', '/forgot-password', '/reset-password']

// Static routes that should bypass middleware
const staticRoutes = ['/_next', '/api', '/uploads', '/images', '/favicon.ico']

function isStaticRoute(pathname: string): boolean {
  return staticRoutes.some((route) => pathname.startsWith(route)) || pathname.includes('.')
}

function isProtectedRoute(pathname: string): boolean {
  // Remove locale prefix for checking
  const pathWithoutLocale = pathname.replace(/^\/(az|en|tr)/, '') || '/'
  return protectedRoutes.some((route) => pathWithoutLocale.startsWith(route))
}

function isAuthRoute(pathname: string): boolean {
  const pathWithoutLocale = pathname.replace(/^\/(az|en|tr)/, '') || '/'
  return authRoutes.some((route) => pathWithoutLocale.startsWith(route))
}

function getLocaleFromPath(pathname: string): string {
  const match = pathname.match(/^\/(az|en|tr)/)
  return match ? match[1] : routing.defaultLocale
}

export default async function middleware(request: NextRequest) {
  const { pathname } = request.nextUrl

  // Skip static routes
  if (isStaticRoute(pathname)) {
    // Add CORS headers for API routes
    if (pathname.startsWith('/api')) {
      const response = NextResponse.next()
      response.headers.set('Access-Control-Allow-Origin', process.env.CORS_ORIGINS || '*')
      response.headers.set('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, PATCH, OPTIONS')
      response.headers.set('Access-Control-Allow-Headers', 'Content-Type, Authorization')
      response.headers.set('Access-Control-Allow-Credentials', 'true')
      
      if (request.method === 'OPTIONS') {
        return new NextResponse(null, { status: 200, headers: response.headers })
      }
      return response
    }
    return NextResponse.next()
  }

  // Get auth token
  const authToken = request.cookies.get('auth-token')?.value
  const isAuthenticated = !!authToken
  const locale = getLocaleFromPath(pathname)

  // Redirect unauthenticated users from protected routes
  if (!isAuthenticated && isProtectedRoute(pathname)) {
    const loginUrl = new URL(`/${locale}/login`, request.url)
    loginUrl.searchParams.set('callbackUrl', pathname)
    return NextResponse.redirect(loginUrl)
  }

  // Redirect authenticated users from auth routes
  if (isAuthenticated && isAuthRoute(pathname)) {
    return NextResponse.redirect(new URL(`/${locale}/dashboard`, request.url))
  }

  // Apply next-intl middleware for locale handling
  return intlMiddleware(request)
}

export const config = {
  // Match all paths except static files
  matcher: ['/((?!_next|api|uploads|images|.*\\..*).*)'],
}
