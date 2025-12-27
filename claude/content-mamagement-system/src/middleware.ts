import { NextResponse, type NextRequest } from 'next/server'
import createIntlMiddleware from 'next-intl/middleware'
import { locales, defaultLocale } from '@/lib/i18n/config'

const intlMiddleware = createIntlMiddleware({
  locales,
  defaultLocale,
  localePrefix: 'always',
})

const publicRoutes = ['/', '/login', '/register', '/forgot-password', '/reset-password', '/blog', '/about', '/contact']
const adminRoutes = ['/admin']
const protectedRoutes = ['/dashboard', '/chat', '/profile']

function isPublicRoute(pathname: string): boolean {
  const pathWithoutLocale = pathname.replace(/^\/(az|en|tr)/, '') || '/'
  return publicRoutes.some((route) => {
    if (route === '/') return pathWithoutLocale === '/'
    return pathWithoutLocale.startsWith(route)
  })
}

function isAdminRoute(pathname: string): boolean {
  const pathWithoutLocale = pathname.replace(/^\/(az|en|tr)/, '') || '/'
  return adminRoutes.some((route) => pathWithoutLocale.startsWith(route))
}

function isProtectedRoute(pathname: string): boolean {
  const pathWithoutLocale = pathname.replace(/^\/(az|en|tr)/, '') || '/'
  return protectedRoutes.some((route) => pathWithoutLocale.startsWith(route))
}

function isStaticRoute(pathname: string): boolean {
  return (
    pathname.startsWith('/_next') ||
    pathname.startsWith('/api') ||
    pathname.startsWith('/uploads') ||
    pathname.startsWith('/images') ||
    pathname.includes('.')
  )
}

export async function middleware(request: NextRequest) {
  const { pathname } = request.nextUrl

  if (isStaticRoute(pathname)) {
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

  const authToken = request.cookies.get('auth-token')?.value
  const isAuthenticated = !!authToken

  if (!isAuthenticated && (isProtectedRoute(pathname) || isAdminRoute(pathname))) {
    const locale = pathname.split('/')[1] || defaultLocale
    const loginUrl = new URL(`/${locale}/login`, request.url)
    loginUrl.searchParams.set('callbackUrl', pathname)
    return NextResponse.redirect(loginUrl)
  }

  if (isAuthenticated && (pathname.includes('/login') || pathname.includes('/register'))) {
    const locale = pathname.split('/')[1] || defaultLocale
    return NextResponse.redirect(new URL(`/${locale}/dashboard`, request.url))
  }

  return intlMiddleware(request)
}

export const config = {
  matcher: ['/((?!api|_next|uploads|images|.*\\..*).*)'],
}
