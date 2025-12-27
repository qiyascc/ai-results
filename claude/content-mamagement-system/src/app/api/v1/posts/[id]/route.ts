import { NextRequest } from 'next/server'
import { db } from '@/lib/db'
import { getCurrentUser } from '@/lib/auth/jwt'
import { postUpdateSchema } from '@/schemas'
import { responses, handleApiError, validationErrorResponse } from '@/lib/api/response'
import { generateSlug } from '@/lib/utils'

interface RouteParams {
  params: { id: string }
}

export async function GET(request: NextRequest, { params }: RouteParams) {
  try {
    const { id } = await params

    const post = await db.post.findUnique({
      where: { id },
      include: {
        author: { select: { id: true, name: true, avatar: true } },
        category: { select: { id: true, name: true, slug: true } },
        tags: { include: { tag: true } },
      },
    })

    if (!post) {
      return responses.notFound('Post not found')
    }

    return responses.ok({ post })
  } catch (error) {
    return handleApiError(error)
  }
}

export async function PUT(request: NextRequest, { params }: RouteParams) {
  try {
    const user = await getCurrentUser()
    if (!user) {
      return responses.unauthorized()
    }

    const { id } = await params
    const body = await request.json()
    
    const result = postUpdateSchema.safeParse(body)
    if (!result.success) {
      return validationErrorResponse(result.error)
    }

    const existingPost = await db.post.findUnique({ where: { id } })
    if (!existingPost) {
      return responses.notFound('Post not found')
    }

    // Check ownership or admin
    if (existingPost.authorId !== user.sub && user.role !== 'super-admin' && user.role !== 'admin') {
      return responses.forbidden('You can only edit your own posts')
    }

    const { title, content, excerpt, coverImage, categoryId, tags, status, locale, metaTitle, metaDescription } = result.data

    const updateData: Record<string, unknown> = {}
    
    if (title !== undefined) {
      updateData.title = title
      updateData.slug = generateSlug(title)
    }
    if (content !== undefined) updateData.content = content
    if (excerpt !== undefined) updateData.excerpt = excerpt
    if (coverImage !== undefined) updateData.coverImage = coverImage
    if (categoryId !== undefined) updateData.categoryId = categoryId
    if (locale !== undefined) updateData.locale = locale
    if (metaTitle !== undefined) updateData.metaTitle = metaTitle
    if (metaDescription !== undefined) updateData.metaDescription = metaDescription
    
    if (status !== undefined) {
      updateData.status = status
      if (status === 'PUBLISHED' && !existingPost.publishedAt) {
        updateData.publishedAt = new Date()
      }
    }

    // Handle tags
    if (tags !== undefined) {
      await db.postTag.deleteMany({ where: { postId: id } })
      if (tags.length > 0) {
        await db.postTag.createMany({
          data: tags.map((tagId: string) => ({ postId: id, tagId })),
        })
      }
    }

    const post = await db.post.update({
      where: { id },
      data: updateData,
      include: {
        author: { select: { id: true, name: true, avatar: true } },
        category: { select: { id: true, name: true, slug: true } },
        tags: { include: { tag: true } },
      },
    })

    return responses.ok({ post }, 'Post updated successfully')
  } catch (error) {
    return handleApiError(error)
  }
}

export async function DELETE(request: NextRequest, { params }: RouteParams) {
  try {
    const user = await getCurrentUser()
    if (!user) {
      return responses.unauthorized()
    }

    const { id } = await params

    const existingPost = await db.post.findUnique({ where: { id } })
    if (!existingPost) {
      return responses.notFound('Post not found')
    }

    // Check ownership or admin
    if (existingPost.authorId !== user.sub && user.role !== 'super-admin' && user.role !== 'admin') {
      return responses.forbidden('You can only delete your own posts')
    }

    await db.postTag.deleteMany({ where: { postId: id } })
    await db.post.delete({ where: { id } })

    return responses.ok({ success: true }, 'Post deleted successfully')
  } catch (error) {
    return handleApiError(error)
  }
}
