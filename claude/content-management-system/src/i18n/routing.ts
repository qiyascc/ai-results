import { defineRouting } from 'next-intl/routing'

export const locales = ['az', 'en', 'tr'] as const
export type Locale = (typeof locales)[number]

export const defaultLocale: Locale = 'az'

export const routing = defineRouting({
  locales,
  defaultLocale,
  localePrefix: 'always',
})

// Locale metadata
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
  az: 'az',
  en: 'en',
  tr: 'tr',
}
