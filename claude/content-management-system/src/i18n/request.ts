import { getRequestConfig } from 'next-intl/server'
import { routing, type Locale } from './routing'

export default getRequestConfig(async ({ requestLocale }) => {
  // This typically corresponds to the `[locale]` segment
  let locale = await requestLocale

  // Ensure that the incoming locale is valid
  if (!locale || !routing.locales.includes(locale as Locale)) {
    locale = routing.defaultLocale
  }

  // Load all translation namespaces
  const messages = {
    common: (await import(`@/locales/${locale}/common.json`)).default,
    auth: (await import(`@/locales/${locale}/auth.json`)).default,
    admin: (await import(`@/locales/${locale}/admin.json`)).default,
    dashboard: (await import(`@/locales/${locale}/dashboard.json`)).default,
    errors: (await import(`@/locales/${locale}/errors.json`)).default,
    validation: (await import(`@/locales/${locale}/validation.json`)).default,
    blog: (await import(`@/locales/${locale}/blog.json`)).default,
  }

  return {
    locale,
    messages,
    timeZone: 'Asia/Baku',
    now: new Date(),
  }
})
