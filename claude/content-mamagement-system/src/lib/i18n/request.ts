import { getRequestConfig } from 'next-intl/server'
import { notFound } from 'next/navigation'
import { locales, defaultLocale, type Locale } from './config'

export default getRequestConfig(async ({ locale }) => {
  if (!locales.includes(locale as Locale)) {
    notFound()
  }

  const messages = await loadMessages(locale as Locale)

  return {
    messages,
    timeZone: 'Asia/Baku',
    now: new Date(),
  }
})

async function loadMessages(locale: Locale): Promise<Record<string, unknown>> {
  const namespaces = ['common', 'auth', 'dashboard', 'admin', 'errors', 'validation', 'blog']
  const messages: Record<string, unknown> = {}
  
  for (const namespace of namespaces) {
    try {
      const module = await import(`@/locales/${locale}/${namespace}.json`)
      messages[namespace] = module.default
    } catch {
      if (locale !== defaultLocale) {
        try {
          const fallbackModule = await import(`@/locales/${defaultLocale}/${namespace}.json`)
          messages[namespace] = fallbackModule.default
        } catch {
          messages[namespace] = {}
        }
      }
    }
  }
  
  return messages
}
