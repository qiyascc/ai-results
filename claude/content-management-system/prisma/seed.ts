import { PrismaClient, UserStatus, PostStatus, TextDirection, SettingType } from '@prisma/client'
import bcrypt from 'bcryptjs'

const prisma = new PrismaClient()

async function main() {
  console.log('🌱 Starting database seed...\n')

  // ==================== PERMISSIONS ====================
  console.log('📝 Creating permissions...')
  
  const permissionsData = [
    // Users
    { name: 'users:create', resource: 'users', action: 'create', description: 'Create users' },
    { name: 'users:read', resource: 'users', action: 'read', description: 'View users' },
    { name: 'users:update', resource: 'users', action: 'update', description: 'Update users' },
    { name: 'users:delete', resource: 'users', action: 'delete', description: 'Delete users' },
    // Roles
    { name: 'roles:create', resource: 'roles', action: 'create', description: 'Create roles' },
    { name: 'roles:read', resource: 'roles', action: 'read', description: 'View roles' },
    { name: 'roles:update', resource: 'roles', action: 'update', description: 'Update roles' },
    { name: 'roles:delete', resource: 'roles', action: 'delete', description: 'Delete roles' },
    // Posts
    { name: 'posts:create', resource: 'posts', action: 'create', description: 'Create posts' },
    { name: 'posts:read', resource: 'posts', action: 'read', description: 'View posts' },
    { name: 'posts:update', resource: 'posts', action: 'update', description: 'Update any posts' },
    { name: 'posts:delete', resource: 'posts', action: 'delete', description: 'Delete any posts' },
    { name: 'posts:publish', resource: 'posts', action: 'publish', description: 'Publish posts' },
    { name: 'posts:update:own', resource: 'posts', action: 'update:own', description: 'Update own posts' },
    { name: 'posts:delete:own', resource: 'posts', action: 'delete:own', description: 'Delete own posts' },
    // Categories
    { name: 'categories:create', resource: 'categories', action: 'create', description: 'Create categories' },
    { name: 'categories:read', resource: 'categories', action: 'read', description: 'View categories' },
    { name: 'categories:update', resource: 'categories', action: 'update', description: 'Update categories' },
    { name: 'categories:delete', resource: 'categories', action: 'delete', description: 'Delete categories' },
    // Tags
    { name: 'tags:create', resource: 'tags', action: 'create', description: 'Create tags' },
    { name: 'tags:read', resource: 'tags', action: 'read', description: 'View tags' },
    { name: 'tags:update', resource: 'tags', action: 'update', description: 'Update tags' },
    { name: 'tags:delete', resource: 'tags', action: 'delete', description: 'Delete tags' },
    // Media
    { name: 'media:create', resource: 'media', action: 'create', description: 'Upload media' },
    { name: 'media:read', resource: 'media', action: 'read', description: 'View media' },
    { name: 'media:update', resource: 'media', action: 'update', description: 'Update media' },
    { name: 'media:delete', resource: 'media', action: 'delete', description: 'Delete media' },
    // Languages
    { name: 'languages:manage', resource: 'languages', action: 'manage', description: 'Manage languages' },
    // Translations
    { name: 'translations:manage', resource: 'translations', action: 'manage', description: 'Manage translations' },
    // Settings
    { name: 'settings:read', resource: 'settings', action: 'read', description: 'View settings' },
    { name: 'settings:update', resource: 'settings', action: 'update', description: 'Update settings' },
    // Contacts
    { name: 'contacts:read', resource: 'contacts', action: 'read', description: 'View contacts' },
    { name: 'contacts:update', resource: 'contacts', action: 'update', description: 'Update contacts' },
    { name: 'contacts:delete', resource: 'contacts', action: 'delete', description: 'Delete contacts' },
    // Audit
    { name: 'audit:read', resource: 'audit', action: 'read', description: 'View audit logs' },
    // Chat
    { name: 'chat:access', resource: 'chat', action: 'access', description: 'Access AI chat' },
    // Analytics
    { name: 'analytics:read', resource: 'analytics', action: 'read', description: 'View analytics' },
  ]

  for (const perm of permissionsData) {
    await prisma.permission.upsert({
      where: { name: perm.name },
      update: {},
      create: perm,
    })
  }
  console.log(`   ✅ Created ${permissionsData.length} permissions`)

  // ==================== ROLES ====================
  console.log('👥 Creating roles...')

  const allPermissions = await prisma.permission.findMany()
  const permMap = Object.fromEntries(allPermissions.map((p) => [p.name, p.id]))

  const rolesData = [
    {
      name: 'Super Admin',
      slug: 'super_admin',
      description: 'Full system access - can do everything',
      isSystem: true,
      permissions: allPermissions.map((p) => p.name),
    },
    {
      name: 'Admin',
      slug: 'admin',
      description: 'Administrative access - manage users and content',
      isSystem: true,
      permissions: [
        'users:create', 'users:read', 'users:update',
        'roles:read',
        'posts:create', 'posts:read', 'posts:update', 'posts:delete', 'posts:publish',
        'categories:create', 'categories:read', 'categories:update', 'categories:delete',
        'tags:create', 'tags:read', 'tags:update', 'tags:delete',
        'media:create', 'media:read', 'media:update', 'media:delete',
        'languages:manage', 'translations:manage',
        'settings:read',
        'contacts:read', 'contacts:update', 'contacts:delete',
        'audit:read', 'chat:access', 'analytics:read',
      ],
    },
    {
      name: 'Editor',
      slug: 'editor',
      description: 'Content management - can edit and publish content',
      isSystem: true,
      permissions: [
        'posts:create', 'posts:read', 'posts:update', 'posts:publish',
        'categories:read',
        'tags:create', 'tags:read',
        'media:create', 'media:read',
        'chat:access',
      ],
    },
    {
      name: 'Author',
      slug: 'author',
      description: 'Can create and manage own content',
      isSystem: true,
      permissions: [
        'posts:create', 'posts:read', 'posts:update:own', 'posts:delete:own',
        'categories:read',
        'tags:read',
        'media:create', 'media:read',
        'chat:access',
      ],
    },
    {
      name: 'User',
      slug: 'user',
      description: 'Basic user - read-only access with chat',
      isSystem: true,
      permissions: [
        'posts:read',
        'categories:read',
        'tags:read',
        'chat:access',
      ],
    },
  ]

  for (const roleData of rolesData) {
    const { permissions, ...role } = roleData
    
    const existingRole = await prisma.role.findUnique({ where: { slug: role.slug } })
    
    if (existingRole) {
      await prisma.rolePermission.deleteMany({ where: { roleId: existingRole.id } })
      await prisma.rolePermission.createMany({
        data: permissions.filter(p => permMap[p]).map((p) => ({
          roleId: existingRole.id,
          permissionId: permMap[p],
        })),
      })
    } else {
      const newRole = await prisma.role.create({ data: role })
      await prisma.rolePermission.createMany({
        data: permissions.filter(p => permMap[p]).map((p) => ({
          roleId: newRole.id,
          permissionId: permMap[p],
        })),
      })
    }
  }
  console.log(`   ✅ Created ${rolesData.length} roles`)

  // ==================== LANGUAGES ====================
  console.log('🌍 Creating languages...')

  const languagesData = [
    { code: 'az', name: 'Azerbaijani', nativeName: 'Azərbaycan', flag: '🇦🇿', direction: TextDirection.LTR, isDefault: true, sortOrder: 1 },
    { code: 'en', name: 'English', nativeName: 'English', flag: '🇬🇧', direction: TextDirection.LTR, isDefault: false, sortOrder: 2 },
    { code: 'tr', name: 'Turkish', nativeName: 'Türkçe', flag: '🇹🇷', direction: TextDirection.LTR, isDefault: false, sortOrder: 3 },
  ]

  for (const lang of languagesData) {
    await prisma.language.upsert({
      where: { code: lang.code },
      update: lang,
      create: lang,
    })
  }
  console.log(`   ✅ Created ${languagesData.length} languages`)

  // ==================== ADMIN USER ====================
  console.log('👤 Creating admin user...')

  const superAdminRole = await prisma.role.findUnique({ where: { slug: 'super_admin' } })
  if (!superAdminRole) throw new Error('Super admin role not found')

  const hashedPassword = await bcrypt.hash('admin123', 12)

  const adminUser = await prisma.user.upsert({
    where: { email: 'admin@qiyas.cc' },
    update: {},
    create: {
      email: 'admin@qiyas.cc',
      name: 'System Administrator',
      password: hashedPassword,
      roleId: superAdminRole.id,
      status: UserStatus.ACTIVE,
      locale: 'az',
      emailVerified: new Date(),
    },
  })
  console.log('   ✅ Created admin user')

  // ==================== SAMPLE USERS ====================
  console.log('👥 Creating sample users...')

  const editorRole = await prisma.role.findUnique({ where: { slug: 'editor' } })
  const authorRole = await prisma.role.findUnique({ where: { slug: 'author' } })
  const userRole = await prisma.role.findUnique({ where: { slug: 'user' } })

  const sampleUsers = [
    { email: 'editor@qiyas.cc', name: 'Editor User', roleId: editorRole!.id },
    { email: 'author@qiyas.cc', name: 'Author User', roleId: authorRole!.id },
    { email: 'user@qiyas.cc', name: 'Regular User', roleId: userRole!.id },
  ]

  for (const userData of sampleUsers) {
    await prisma.user.upsert({
      where: { email: userData.email },
      update: {},
      create: {
        ...userData,
        password: hashedPassword,
        status: UserStatus.ACTIVE,
        locale: 'az',
        emailVerified: new Date(),
      },
    })
  }
  console.log(`   ✅ Created ${sampleUsers.length} sample users`)

  // ==================== CATEGORIES ====================
  console.log('📁 Creating categories...')

  const azLang = await prisma.language.findUnique({ where: { code: 'az' } })
  const enLang = await prisma.language.findUnique({ where: { code: 'en' } })
  const trLang = await prisma.language.findUnique({ where: { code: 'tr' } })

  const categoriesData = [
    {
      slug: 'technology',
      sortOrder: 1,
      translations: [
        { languageId: azLang!.id, name: 'Texnologiya', description: 'Texnologiya xəbərləri və məqalələri' },
        { languageId: enLang!.id, name: 'Technology', description: 'Technology news and articles' },
        { languageId: trLang!.id, name: 'Teknoloji', description: 'Teknoloji haberleri ve makaleleri' },
      ],
    },
    {
      slug: 'business',
      sortOrder: 2,
      translations: [
        { languageId: azLang!.id, name: 'Biznes', description: 'Biznes xəbərləri və analitika' },
        { languageId: enLang!.id, name: 'Business', description: 'Business news and analytics' },
        { languageId: trLang!.id, name: 'İş Dünyası', description: 'İş dünyası haberleri ve analizler' },
      ],
    },
    {
      slug: 'lifestyle',
      sortOrder: 3,
      translations: [
        { languageId: azLang!.id, name: 'Həyat Tərzi', description: 'Həyat tərzi və sağlamlıq' },
        { languageId: enLang!.id, name: 'Lifestyle', description: 'Lifestyle and wellness' },
        { languageId: trLang!.id, name: 'Yaşam', description: 'Yaşam tarzı ve sağlık' },
      ],
    },
    {
      slug: 'education',
      sortOrder: 4,
      translations: [
        { languageId: azLang!.id, name: 'Təhsil', description: 'Təhsil və öyrənmə' },
        { languageId: enLang!.id, name: 'Education', description: 'Education and learning' },
        { languageId: trLang!.id, name: 'Eğitim', description: 'Eğitim ve öğrenme' },
      ],
    },
  ]

  for (const catData of categoriesData) {
    const { translations, ...category } = catData
    const existing = await prisma.category.findUnique({ where: { slug: category.slug } })
    
    if (!existing) {
      await prisma.category.create({
        data: {
          ...category,
          translations: { create: translations },
        },
      })
    }
  }
  console.log(`   ✅ Created ${categoriesData.length} categories`)

  // ==================== TAGS ====================
  console.log('🏷️ Creating tags...')

  const tagsData = [
    { name: 'Next.js', slug: 'nextjs' },
    { name: 'React', slug: 'react' },
    { name: 'TypeScript', slug: 'typescript' },
    { name: 'JavaScript', slug: 'javascript' },
    { name: 'Node.js', slug: 'nodejs' },
    { name: 'AI', slug: 'ai' },
    { name: 'Machine Learning', slug: 'machine-learning' },
    { name: 'Web Development', slug: 'web-development' },
    { name: 'Tutorial', slug: 'tutorial' },
    { name: 'News', slug: 'news' },
  ]

  for (const tag of tagsData) {
    await prisma.tag.upsert({
      where: { slug: tag.slug },
      update: {},
      create: tag,
    })
  }
  console.log(`   ✅ Created ${tagsData.length} tags`)

  // ==================== SAMPLE POSTS ====================
  console.log('📝 Creating sample posts...')

  const techCategory = await prisma.category.findUnique({ where: { slug: 'technology' } })
  const nextjsTag = await prisma.tag.findUnique({ where: { slug: 'nextjs' } })
  const reactTag = await prisma.tag.findUnique({ where: { slug: 'react' } })

  const samplePost = await prisma.post.upsert({
    where: { slug: 'welcome-to-qiyas-cms' },
    update: {},
    create: {
      slug: 'welcome-to-qiyas-cms',
      status: PostStatus.PUBLISHED,
      featured: true,
      authorId: adminUser.id,
      publishedAt: new Date(),
      translations: {
        create: [
          {
            languageId: azLang!.id,
            title: 'Qiyas CMS-ə Xoş Gəlmisiniz',
            excerpt: 'Multi-language dəstəkli, AI-powered chat və kapsamlı admin paneli olan modern CMS sistemi.',
            content: `
# Qiyas CMS-ə Xoş Gəlmisiniz

Bu, **Qiyas CMS** ilə yaradılmış ilk yazıdır. Bu sistem aşağıdakı xüsusiyyətlərə malikdir:

## Xüsusiyyətlər

- 🌍 **Multi-Language** - AZ, EN, TR dil dəstəyi
- 🔐 **RBAC** - Rol əsaslı giriş kontrolu
- 📝 **Rich Editor** - TipTap ilə zəngin mətn redaktoru
- 💬 **AI Chat** - Süni intellekt ilə söhbət
- 📁 **Media Library** - Şəkil idarəetməsi

## Başlamaq

Admin panelinə daxil olun və kontenti idarə etməyə başlayın!
            `.trim(),
            metaTitle: 'Qiyas CMS-ə Xoş Gəlmisiniz',
            metaDescription: 'Multi-language dəstəkli modern CMS sistemi',
          },
          {
            languageId: enLang!.id,
            title: 'Welcome to Qiyas CMS',
            excerpt: 'A modern CMS with multi-language support, AI-powered chat, and comprehensive admin panel.',
            content: `
# Welcome to Qiyas CMS

This is the first post created with **Qiyas CMS**. This system includes the following features:

## Features

- 🌍 **Multi-Language** - AZ, EN, TR language support
- 🔐 **RBAC** - Role-based access control
- 📝 **Rich Editor** - TipTap rich text editor
- 💬 **AI Chat** - AI-powered chat functionality
- 📁 **Media Library** - Image management

## Getting Started

Log in to the admin panel and start managing your content!
            `.trim(),
            metaTitle: 'Welcome to Qiyas CMS',
            metaDescription: 'A modern CMS with multi-language support',
          },
          {
            languageId: trLang!.id,
            title: 'Qiyas CMS\'e Hoş Geldiniz',
            excerpt: 'Çok dilli destek, yapay zeka destekli sohbet ve kapsamlı yönetim paneli olan modern CMS.',
            content: `
# Qiyas CMS'e Hoş Geldiniz

Bu, **Qiyas CMS** ile oluşturulmuş ilk yazıdır. Bu sistem aşağıdaki özelliklere sahiptir:

## Özellikler

- 🌍 **Çoklu Dil** - AZ, EN, TR dil desteği
- 🔐 **RBAC** - Rol tabanlı erişim kontrolü
- 📝 **Zengin Editör** - TipTap zengin metin editörü
- 💬 **AI Sohbet** - Yapay zeka destekli sohbet
- 📁 **Medya Kütüphanesi** - Görsel yönetimi

## Başlarken

Yönetim paneline giriş yapın ve içeriklerinizi yönetmeye başlayın!
            `.trim(),
            metaTitle: 'Qiyas CMS\'e Hoş Geldiniz',
            metaDescription: 'Çok dilli destekli modern CMS sistemi',
          },
        ],
      },
      categories: {
        create: [{ categoryId: techCategory!.id }],
      },
      tags: {
        create: [
          { tagId: nextjsTag!.id },
          { tagId: reactTag!.id },
        ],
      },
    },
  })
  console.log('   ✅ Created sample post')

  // ==================== SETTINGS ====================
  console.log('⚙️ Creating settings...')

  const settingsData = [
    { key: 'site_name', value: 'Qiyas CMS', type: SettingType.STRING, group: 'general', isPublic: true },
    { key: 'site_description', value: 'Modern content management system', type: SettingType.STRING, group: 'general', isPublic: true },
    { key: 'site_url', value: 'http://localhost:3000', type: SettingType.STRING, group: 'general', isPublic: true },
    { key: 'default_locale', value: 'az', type: SettingType.STRING, group: 'general', isPublic: true },
    { key: 'timezone', value: 'Asia/Baku', type: SettingType.STRING, group: 'general', isPublic: false },
    { key: 'posts_per_page', value: 10, type: SettingType.NUMBER, group: 'general', isPublic: true },
    { key: 'allow_registration', value: true, type: SettingType.BOOLEAN, group: 'auth', isPublic: false },
    { key: 'email_verification', value: true, type: SettingType.BOOLEAN, group: 'auth', isPublic: false },
    { key: 'ai_provider', value: 'anthropic', type: SettingType.STRING, group: 'ai', isPublic: false },
    { key: 'ai_model', value: 'claude-3-sonnet-20240229', type: SettingType.STRING, group: 'ai', isPublic: false },
    { key: 'ai_temperature', value: 0.7, type: SettingType.NUMBER, group: 'ai', isPublic: false },
    { key: 'max_upload_size', value: 10485760, type: SettingType.NUMBER, group: 'media', isPublic: false },
    { key: 'allowed_file_types', value: 'image/jpeg,image/png,image/gif,image/webp,application/pdf', type: SettingType.STRING, group: 'media', isPublic: false },
  ]

  for (const setting of settingsData) {
    await prisma.setting.upsert({
      where: { key: setting.key },
      update: { value: setting.value },
      create: setting,
    })
  }
  console.log(`   ✅ Created ${settingsData.length} settings`)

  // ==================== DONE ====================
  console.log('\n✨ Database seeding completed!\n')
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
  console.log('📧 Admin Credentials:')
  console.log('   Email:    admin@qiyas.cc')
  console.log('   Password: admin123')
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
  console.log('📧 Sample Users: (password: admin123)')
  console.log('   Editor:   editor@qiyas.cc')
  console.log('   Author:   author@qiyas.cc')
  console.log('   User:     user@qiyas.cc')
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n')
}

main()
  .catch((e) => {
    console.error('❌ Seed error:', e)
    process.exit(1)
  })
  .finally(async () => {
    await prisma.$disconnect()
  })
