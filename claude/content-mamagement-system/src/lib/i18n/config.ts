export const locales = ['az', 'en', 'tr'] as const
export type Locale = (typeof locales)[number]

export const defaultLocale: Locale = 'az'

export const localeNames: Record<Locale, string> = {
  az: 'Azərbaycan',
  en: 'English',
  tr: 'Türkçe',
}

export const localeFlags: Record<Locale, string> = {
  az: '🇦🇿',
  en: '🇬🇧',
  tr: '🇹🇷',
}

export const localeDirections: Record<Locale, 'ltr' | 'rtl'> = {
  az: 'ltr',
  en: 'ltr',
  tr: 'ltr',
}

export const localeHtmlLang: Record<Locale, string> = {
  az: 'az-AZ',
  en: 'en-US',
  tr: 'tr-TR',
}

export function isValidLocale(locale: string): locale is Locale {
  return locales.includes(locale as Locale)
}

export function getLocaleFromPathname(pathname: string): Locale | null {
  const segments = pathname.split('/')
  const potentialLocale = segments[1]
  if (isValidLocale(potentialLocale)) return potentialLocale
  return null
}

export function removeLocaleFromPathname(pathname: string): string {
  const locale = getLocaleFromPathname(pathname)
  if (locale) return pathname.replace(`/${locale}`, '') || '/'
  return pathname
}

export function addLocaleToPathname(pathname: string, locale: Locale): string {
  const cleanPath = removeLocaleFromPathname(pathname)
  return `/${locale}${cleanPath === '/' ? '' : cleanPath}`
}
