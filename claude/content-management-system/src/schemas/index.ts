import { z } from 'zod'

// ==================== COMMON ====================

export const paginationSchema = z.object({
  page: z.coerce.number().int().positive().default(1),
  limit: z.coerce.number().int().positive().max(100).default(15),
  sortBy: z.string().optional(),
  sortOrder: z.enum(['asc', 'desc']).default('desc'),
})

export const idSchema = z.object({ id: z.string().min(1) })
export const slugSchema = z.object({ slug: z.string().min(1) })

// ==================== AUTH ====================

export const loginSchema = z.object({
  email: z.string().email('Invalid email address'),
  password: z.string().min(1, 'Password is required'),
  rememberMe: z.boolean().optional().default(false),
})

export const registerSchema = z.object({
  name: z.string().min(2, 'Name must be at least 2 characters'),
  email: z.string().email('Invalid email address'),
  password: z.string()
    .min(8, 'Password must be at least 8 characters')
    .regex(/[a-z]/, 'Password must contain a lowercase letter')
    .regex(/[A-Z]/, 'Password must contain an uppercase letter')
    .regex(/\d/, 'Password must contain a number'),
  confirmPassword: z.string(),
  locale: z.enum(['az', 'en', 'tr']).optional().default('az'),
}).refine((data) => data.password === data.confirmPassword, {
  message: 'Passwords do not match',
  path: ['confirmPassword'],
})

export const forgotPasswordSchema = z.object({
  email: z.string().email('Invalid email address'),
})

export const resetPasswordSchema = z.object({
  token: z.string().min(1),
  password: z.string().min(8).regex(/[a-z]/).regex(/[A-Z]/).regex(/\d/),
  confirmPassword: z.string(),
}).refine((data) => data.password === data.confirmPassword, {
  message: 'Passwords do not match',
  path: ['confirmPassword'],
})

export const changePasswordSchema = z.object({
  currentPassword: z.string().min(1),
  newPassword: z.string().min(8).regex(/[a-z]/).regex(/[A-Z]/).regex(/\d/),
  confirmPassword: z.string(),
}).refine((data) => data.newPassword === data.confirmPassword, {
  message: 'Passwords do not match',
  path: ['confirmPassword'],
})

// ==================== USER ====================

export const createUserSchema = z.object({
  name: z.string().min(2),
  email: z.string().email(),
  password: z.string().min(8),
  roleId: z.string().min(1),
  isActive: z.boolean().optional().default(true),
  locale: z.enum(['az', 'en', 'tr']).optional().default('az'),
})

export const updateUserSchema = z.object({
  name: z.string().min(2).optional(),
  email: z.string().email().optional(),
  roleId: z.string().optional(),
  status: z.enum(['ACTIVE', 'INACTIVE', 'SUSPENDED', 'PENDING']).optional(),
  locale: z.enum(['az', 'en', 'tr']).optional(),
  avatar: z.string().url().nullable().optional(),
  bio: z.string().max(500).optional(),
})

export const updateProfileSchema = z.object({
  name: z.string().min(2).optional(),
  bio: z.string().max(500).optional(),
  locale: z.enum(['az', 'en', 'tr']).optional(),
  avatar: z.string().url().nullable().optional(),
})

// ==================== ROLE ====================

export const createRoleSchema = z.object({
  name: z.string().min(2),
  slug: z.string().regex(/^[a-z0-9_]+$/).optional(),
  description: z.string().optional(),
  permissions: z.array(z.string()).default([]),
})

export const updateRoleSchema = z.object({
  name: z.string().min(2).optional(),
  description: z.string().optional(),
  permissions: z.array(z.string()).optional(),
})

// ==================== POST ====================

export const postTranslationSchema = z.object({
  languageId: z.string().min(1),
  title: z.string().min(1, 'Title is required'),
  excerpt: z.string().optional(),
  content: z.string().min(1, 'Content is required'),
  metaTitle: z.string().max(60).optional(),
  metaDescription: z.string().max(160).optional(),
  metaKeywords: z.string().optional(),
})

export const createPostSchema = z.object({
  slug: z.string().regex(/^[a-z0-9-]+$/).optional(),
  coverImage: z.string().url().nullable().optional(),
  status: z.enum(['DRAFT', 'PENDING_REVIEW', 'PUBLISHED', 'ARCHIVED']).default('DRAFT'),
  featured: z.boolean().optional().default(false),
  categoryIds: z.array(z.string()).optional().default([]),
  tagIds: z.array(z.string()).optional().default([]),
  translations: z.array(postTranslationSchema).min(1, 'At least one translation required'),
  publishedAt: z.string().datetime().nullable().optional(),
})

export const updatePostSchema = createPostSchema.partial()

// ==================== CATEGORY ====================

export const categoryTranslationSchema = z.object({
  languageId: z.string().min(1),
  name: z.string().min(1),
  description: z.string().optional(),
  metaTitle: z.string().max(60).optional(),
  metaDescription: z.string().max(160).optional(),
})

export const createCategorySchema = z.object({
  slug: z.string().regex(/^[a-z0-9-]+$/).optional(),
  parentId: z.string().nullable().optional(),
  image: z.string().url().nullable().optional(),
  sortOrder: z.number().int().optional().default(0),
  isActive: z.boolean().optional().default(true),
  translations: z.array(categoryTranslationSchema).min(1),
})

export const updateCategorySchema = createCategorySchema.partial()

// ==================== TAG ====================

export const createTagSchema = z.object({
  name: z.string().min(1),
  slug: z.string().regex(/^[a-z0-9-]+$/).optional(),
})

export const updateTagSchema = createTagSchema.partial()

// ==================== CONTACT ====================

export const contactFormSchema = z.object({
  firstName: z.string().min(1),
  lastName: z.string().min(1),
  email: z.string().email(),
  phone: z.string().optional(),
  subject: z.string().min(1),
  message: z.string().min(10),
})

// ==================== LANGUAGE ====================

export const createLanguageSchema = z.object({
  code: z.string().min(2).max(5).regex(/^[a-z]+$/),
  name: z.string().min(1),
  nativeName: z.string().min(1),
  flag: z.string().optional(),
  direction: z.enum(['LTR', 'RTL']).default('LTR'),
  isDefault: z.boolean().optional().default(false),
  isActive: z.boolean().optional().default(true),
  sortOrder: z.number().int().optional().default(0),
})

export const updateLanguageSchema = createLanguageSchema.partial().omit({ code: true })

// ==================== TRANSLATION ====================

export const createTranslationSchema = z.object({
  languageId: z.string().min(1),
  namespace: z.string().min(1),
  key: z.string().min(1),
  value: z.string(),
})

export const updateTranslationSchema = z.object({
  value: z.string().min(1),
})

// ==================== CHAT ====================

export const createChatSessionSchema = z.object({
  title: z.string().optional(),
  model: z.string().optional(),
  systemPrompt: z.string().optional(),
  temperature: z.number().min(0).max(2).optional(),
})

export const sendChatMessageSchema = z.object({
  content: z.string().min(1),
})

// ==================== TYPE EXPORTS ====================

export type LoginInput = z.infer<typeof loginSchema>
export type RegisterInput = z.infer<typeof registerSchema>
export type ForgotPasswordInput = z.infer<typeof forgotPasswordSchema>
export type ResetPasswordInput = z.infer<typeof resetPasswordSchema>
export type ChangePasswordInput = z.infer<typeof changePasswordSchema>
export type CreateUserInput = z.infer<typeof createUserSchema>
export type UpdateUserInput = z.infer<typeof updateUserSchema>
export type UpdateProfileInput = z.infer<typeof updateProfileSchema>
export type CreateRoleInput = z.infer<typeof createRoleSchema>
export type UpdateRoleInput = z.infer<typeof updateRoleSchema>
export type CreatePostInput = z.infer<typeof createPostSchema>
export type UpdatePostInput = z.infer<typeof updatePostSchema>
export type CreateCategoryInput = z.infer<typeof createCategorySchema>
export type UpdateCategoryInput = z.infer<typeof updateCategorySchema>
export type CreateTagInput = z.infer<typeof createTagSchema>
export type UpdateTagInput = z.infer<typeof updateTagSchema>
export type ContactFormInput = z.infer<typeof contactFormSchema>
export type CreateLanguageInput = z.infer<typeof createLanguageSchema>
export type UpdateLanguageInput = z.infer<typeof updateLanguageSchema>
export type CreateTranslationInput = z.infer<typeof createTranslationSchema>
export type UpdateTranslationInput = z.infer<typeof updateTranslationSchema>
export type CreateChatSessionInput = z.infer<typeof createChatSessionSchema>
export type SendChatMessageInput = z.infer<typeof sendChatMessageSchema>
export type PaginationInput = z.infer<typeof paginationSchema>
