// ==================== BASE TYPES ====================

export interface ApiResponse<T = unknown> {
  success: boolean
  message?: string
  data?: T
  error?: ApiError
}

export interface ApiError {
  code: string
  message: string
  details?: Record<string, string[]>
}

export interface PaginationMeta {
  page: number
  limit: number
  total: number
  totalPages: number
  hasNext: boolean
  hasPrev: boolean
}

export interface PaginatedResponse<T> {
  data: T[]
  pagination: PaginationMeta
}

// ==================== AUTH ====================

export interface AuthUser {
  id: string
  email: string
  name: string | null
  avatar: string | null
  role: string
  permissions: string[]
  locale: string
}

export interface LoginResponse {
  success: boolean
  message: string
  user: AuthUser
  accessToken?: string
}

// ==================== USER ====================

export interface User {
  id: string
  email: string
  name: string | null
  avatar: string | null
  bio: string | null
  locale: string
  status: UserStatus
  lastLoginAt: string | null
  role: Role
  createdAt: string
  updatedAt: string
}

export type UserStatus = 'ACTIVE' | 'INACTIVE' | 'SUSPENDED' | 'PENDING'

// ==================== ROLE & PERMISSION ====================

export interface Role {
  id: string
  name: string
  slug: string
  description: string | null
  isSystem: boolean
  permissions: Permission[]
  _count?: { users: number }
}

export interface Permission {
  id: string
  name: string
  resource: string
  action: string
  description: string | null
}

// ==================== POST ====================

export interface Post {
  id: string
  slug: string
  coverImage: string | null
  status: PostStatus
  featured: boolean
  views: number
  author: { id: string; name: string | null; avatar: string | null }
  translations: PostTranslation[]
  categories: Category[]
  tags: Tag[]
  publishedAt: string | null
  createdAt: string
  updatedAt: string
}

export interface PostTranslation {
  id: string
  languageId: string
  language?: Language
  title: string
  excerpt: string | null
  content: string
  metaTitle: string | null
  metaDescription: string | null
  metaKeywords: string | null
}

export type PostStatus = 'DRAFT' | 'PENDING_REVIEW' | 'PUBLISHED' | 'ARCHIVED'

// ==================== CATEGORY ====================

export interface Category {
  id: string
  slug: string
  parentId: string | null
  image: string | null
  sortOrder: number
  isActive: boolean
  translations: CategoryTranslation[]
  parent?: Category | null
  children?: Category[]
  _count?: { posts: number }
}

export interface CategoryTranslation {
  id: string
  languageId: string
  language?: Language
  name: string
  description: string | null
  metaTitle: string | null
  metaDescription: string | null
}

// ==================== TAG ====================

export interface Tag {
  id: string
  name: string
  slug: string
  _count?: { posts: number }
}

// ==================== LANGUAGE ====================

export interface Language {
  id: string
  code: string
  name: string
  nativeName: string
  flag: string | null
  direction: 'LTR' | 'RTL'
  isDefault: boolean
  isActive: boolean
  sortOrder: number
}

// ==================== TRANSLATION ====================

export interface Translation {
  id: string
  languageId: string
  namespace: string
  key: string
  value: string
}

// ==================== MEDIA ====================

export interface Media {
  id: string
  filename: string
  originalName: string
  mimeType: string
  size: number
  path: string
  url: string
  width: number | null
  height: number | null
  altText: string | null
  caption: string | null
  thumbnails: { small?: string; medium?: string; large?: string } | null
  folder: string | null
  uploadedBy: { id: string; name: string | null }
  createdAt: string
}

// ==================== CHAT ====================

export interface ChatSession {
  id: string
  title: string | null
  model: string
  systemPrompt: string | null
  temperature: number
  messages?: ChatMessage[]
  createdAt: string
  updatedAt: string
}

export interface ChatMessage {
  id: string
  role: 'USER' | 'ASSISTANT' | 'SYSTEM'
  content: string
  tokens: number | null
  model: string | null
  createdAt: string
}

// ==================== CONTACT ====================

export interface Contact {
  id: string
  firstName: string
  lastName: string
  email: string
  phone: string | null
  subject: string
  message: string
  status: ContactStatus
  createdAt: string
  updatedAt: string
}

export type ContactStatus = 'NEW' | 'READ' | 'REPLIED' | 'ARCHIVED'

// ==================== NOTIFICATION ====================

export interface Notification {
  id: string
  type: NotificationType
  title: string
  message: string
  link: string | null
  isRead: boolean
  data: Record<string, unknown> | null
  createdAt: string
}

export type NotificationType = 'INFO' | 'SUCCESS' | 'WARNING' | 'ERROR' | 'SYSTEM'

// ==================== SETTING ====================

export interface Setting {
  id: string
  key: string
  value: unknown
  type: SettingType
  group: string
  isPublic: boolean
}

export type SettingType = 'STRING' | 'NUMBER' | 'BOOLEAN' | 'JSON' | 'TEXT'

// ==================== AUDIT ====================

export interface AuditLog {
  id: string
  action: string
  entity: string
  entityId: string | null
  user: { id: string; name: string | null; email: string } | null
  oldValue: Record<string, unknown> | null
  newValue: Record<string, unknown> | null
  ipAddress: string | null
  userAgent: string | null
  metadata: Record<string, unknown> | null
  createdAt: string
}

// ==================== ANALYTICS ====================

export interface AnalyticsOverview {
  totalVisitors: number
  totalPageViews: number
  totalSessions: number
  avgSessionDuration: number
  bounceRate: number
  period: string
}

export interface AnalyticsTrend {
  date: string
  count: number
}
