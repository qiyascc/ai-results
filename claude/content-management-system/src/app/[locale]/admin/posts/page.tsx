'use client'

import { useState } from 'react'
import { useTranslations } from 'next-intl'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { useRouter } from '@/i18n/navigation'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Card, CardContent, CardHeader } from '@/components/ui/card'
import { Plus, Search, MoreHorizontal, Pencil, Trash2, Eye } from 'lucide-react'
import { Skeleton } from '@/components/ui/skeleton'

interface Post {
  id: string
  title: string
  slug: string
  status: string
  viewCount: number
  createdAt: string
  author: { id: string; name: string }
  category?: { id: string; name: string }
}

async function fetchPosts(page: number, search: string) {
  const params = new URLSearchParams({ page: String(page), limit: '10' })
  if (search) params.set('search', search)
  
  const res = await fetch(`/api/v1/posts?${params}`)
  if (!res.ok) throw new Error('Failed to fetch posts')
  return res.json()
}

async function deletePost(id: string) {
  const res = await fetch(`/api/v1/posts/${id}`, { method: 'DELETE' })
  if (!res.ok) throw new Error('Failed to delete post')
  return res.json()
}

export default function AdminPostsPage() {
  const t = useTranslations('admin.posts')
  const tCommon = useTranslations('common')
  const router = useRouter()
  const queryClient = useQueryClient()
  
  const [page, setPage] = useState(1)
  const [search, setSearch] = useState('')

  const { data, isLoading, error } = useQuery({
    queryKey: ['admin', 'posts', page, search],
    queryFn: () => fetchPosts(page, search),
  })

  const deleteMutation = useMutation({
    mutationFn: deletePost,
    onSuccess: () => {
      toast.success(tCommon('messages.deletedSuccessfully'))
      queryClient.invalidateQueries({ queryKey: ['admin', 'posts'] })
    },
    onError: () => {
      toast.error(tCommon('messages.error'))
    },
  })

  const handleDelete = (id: string) => {
    if (confirm(tCommon('messages.confirmDelete'))) {
      deleteMutation.mutate(id)
    }
  }

  const getStatusBadge = (status: string) => {
    const variants: Record<string, 'default' | 'secondary' | 'destructive' | 'outline'> = {
      PUBLISHED: 'default',
      DRAFT: 'secondary',
      PENDING: 'outline',
      ARCHIVED: 'destructive',
    }
    return <Badge variant={variants[status] || 'secondary'}>{t(`status.${status.toLowerCase()}`)}</Badge>
  }

  return (
    <div className="p-6">
      <div className="mb-6 flex items-center justify-between">
        <h1 className="text-3xl font-bold">{t('title')}</h1>
        <Button onClick={() => router.push('/admin/posts/new')}>
          <Plus className="mr-2 h-4 w-4" />
          {t('create')}
        </Button>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-4">
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                placeholder={tCommon('actions.search')}
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="pl-9"
              />
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {[1, 2, 3, 4, 5].map((i) => (
                <Skeleton key={i} className="h-16" />
              ))}
            </div>
          ) : error ? (
            <p className="text-center text-destructive">{tCommon('messages.error')}</p>
          ) : (
            <>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{t('fields.title')}</TableHead>
                    <TableHead>{t('fields.category')}</TableHead>
                    <TableHead>{t('fields.status')}</TableHead>
                    <TableHead className="text-right">{t('fields.viewCount')}</TableHead>
                    <TableHead className="w-[70px]">{tCommon('table.actions')}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {data?.data?.posts?.map((post: Post) => (
                    <TableRow key={post.id}>
                      <TableCell>
                        <div>
                          <p className="font-medium">{post.title}</p>
                          <p className="text-sm text-muted-foreground">{post.author.name}</p>
                        </div>
                      </TableCell>
                      <TableCell>{post.category?.name || '-'}</TableCell>
                      <TableCell>{getStatusBadge(post.status)}</TableCell>
                      <TableCell className="text-right">{post.viewCount}</TableCell>
                      <TableCell>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="icon">
                              <MoreHorizontal className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem onClick={() => router.push(`/blog/${post.slug}`)}>
                              <Eye className="mr-2 h-4 w-4" />
                              {tCommon('actions.view')}
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={() => router.push(`/admin/posts/${post.id}`)}>
                              <Pencil className="mr-2 h-4 w-4" />
                              {tCommon('actions.edit')}
                            </DropdownMenuItem>
                            <DropdownMenuItem
                              onClick={() => handleDelete(post.id)}
                              className="text-destructive"
                            >
                              <Trash2 className="mr-2 h-4 w-4" />
                              {tCommon('actions.delete')}
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>

              {data?.data?.pagination && (
                <div className="mt-4 flex items-center justify-between">
                  <p className="text-sm text-muted-foreground">
                    {tCommon('pagination.showing')} {data.data.posts.length} / {data.data.pagination.total} {tCommon('pagination.items')}
                  </p>
                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      disabled={page === 1}
                      onClick={() => setPage(page - 1)}
                    >
                      {tCommon('pagination.previous')}
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      disabled={page >= data.data.pagination.totalPages}
                      onClick={() => setPage(page + 1)}
                    >
                      {tCommon('pagination.next')}
                    </Button>
                  </div>
                </div>
              )}
            </>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
