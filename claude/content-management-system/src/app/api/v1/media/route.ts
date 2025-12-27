import { NextRequest } from 'next/server'
import { writeFile, mkdir } from 'fs/promises'
import { existsSync } from 'fs'
import path from 'path'
import { nanoid } from 'nanoid'
import { db } from '@/lib/db'
import { getCurrentUser } from '@/lib/auth/jwt'
import { paginationSchema } from '@/schemas'
import { responses, handleApiError } from '@/lib/api/response'

const UPLOAD_DIR = path.join(process.cwd(), 'public', 'uploads')
const MAX_FILE_SIZE = 10 * 1024 * 1024 // 10MB
const ALLOWED_TYPES = [
  'image/jpeg',
  'image/png',
  'image/gif',
  'image/webp',
  'image/svg+xml',
  'application/pdf',
  'video/mp4',
  'video/webm',
]

export async function GET(request: NextRequest) {
  try {
    const user = await getCurrentUser()
    if (!user) {
      return responses.unauthorized()
    }

    const { searchParams } = new URL(request.url)
    
    const result = paginationSchema.safeParse({
      page: searchParams.get('page'),
      limit: searchParams.get('limit'),
      search: searchParams.get('search'),
    })

    const { page = 1, limit = 24, search } = result.data || {}
    const skip = (page - 1) * limit

    const type = searchParams.get('type')

    const where: Record<string, unknown> = {}
    
    if (search) {
      where.OR = [
        { filename: { contains: search, mode: 'insensitive' } },
        { alt: { contains: search, mode: 'insensitive' } },
      ]
    }
    
    if (type) {
      where.mimeType = { startsWith: type }
    }

    const [media, total] = await Promise.all([
      db.media.findMany({
        where,
        include: {
          uploadedBy: { select: { id: true, name: true } },
        },
        orderBy: { createdAt: 'desc' },
        skip,
        take: limit,
      }),
      db.media.count({ where }),
    ])

    return responses.ok({
      media,
      pagination: {
        page,
        limit,
        total,
        totalPages: Math.ceil(total / limit),
      },
    })
  } catch (error) {
    return handleApiError(error)
  }
}

export async function POST(request: NextRequest) {
  try {
    const user = await getCurrentUser()
    if (!user) {
      return responses.unauthorized()
    }

    if (!user.permissions.includes('media:upload') && user.role !== 'super-admin') {
      return responses.forbidden('You do not have permission to upload media')
    }

    const formData = await request.formData()
    const file = formData.get('file') as File | null
    const alt = formData.get('alt') as string | null

    if (!file) {
      return responses.badRequest('No file provided')
    }

    if (file.size > MAX_FILE_SIZE) {
      return responses.badRequest('File size exceeds 10MB limit')
    }

    if (!ALLOWED_TYPES.includes(file.type)) {
      return responses.badRequest('File type not allowed')
    }

    // Ensure upload directory exists
    if (!existsSync(UPLOAD_DIR)) {
      await mkdir(UPLOAD_DIR, { recursive: true })
    }

    // Generate unique filename
    const ext = path.extname(file.name)
    const uniqueFilename = `${nanoid()}${ext}`
    
    // Organize by year/month
    const now = new Date()
    const yearMonth = `${now.getFullYear()}/${String(now.getMonth() + 1).padStart(2, '0')}`
    const uploadPath = path.join(UPLOAD_DIR, yearMonth)
    
    if (!existsSync(uploadPath)) {
      await mkdir(uploadPath, { recursive: true })
    }

    const filePath = path.join(uploadPath, uniqueFilename)
    const publicUrl = `/uploads/${yearMonth}/${uniqueFilename}`

    // Write file
    const bytes = await file.arrayBuffer()
    const buffer = Buffer.from(bytes)
    await writeFile(filePath, buffer)

    // Save to database
    const media = await db.media.create({
      data: {
        filename: file.name,
        path: publicUrl,
        mimeType: file.type,
        size: file.size,
        alt: alt || file.name,
        uploadedById: user.sub,
      },
      include: {
        uploadedBy: { select: { id: true, name: true } },
      },
    })

    return responses.created({ media }, 'File uploaded successfully')
  } catch (error) {
    return handleApiError(error)
  }
}
