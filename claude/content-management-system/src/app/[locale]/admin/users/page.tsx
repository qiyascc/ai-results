'use client'

import { useState } from 'react'
import { useTranslations } from 'next-intl'
import { useQuery } from '@tanstack/react-query'
import { useRouter } from '@/i18n/navigation'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
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
import { Plus, Search, MoreHorizontal, Pencil, Trash2 } from 'lucide-react'
import { Skeleton } from '@/components/ui/skeleton'
import { getInitials } from '@/lib/utils'

interface User {
  id: string
  name: string
  email: string
  avatar?: string
  status: string
  createdAt: string
  lastLoginAt?: string
  role: { id: string; name: string; slug: string }
}

async function fetchUsers(page: number, search: string) {
  const params = new URLSearchParams({ page: String(page), limit: '10' })
  if (search) params.set('search', search)
  
  const res = await fetch(`/api/v1/users?${params}`)
  if (!res.ok) throw new Error('Failed to fetch users')
  return res.json()
}

export default function AdminUsersPage() {
  const t = useTranslations('admin.users')
  const tCommon = useTranslations('common')
  const router = useRouter()
  
  const [page, setPage] = useState(1)
  const [search, setSearch] = useState('')

  const { data, isLoading, error } = useQuery({
    queryKey: ['admin', 'users', page, search],
    queryFn: () => fetchUsers(page, search),
  })

  const getStatusBadge = (status: string) => {
    const variants: Record<string, 'default' | 'secondary' | 'destructive' | 'outline'> = {
      ACTIVE: 'default',
      INACTIVE: 'secondary',
      SUSPENDED: 'destructive',
      PENDING: 'outline',
    }
    return <Badge variant={variants[status] || 'secondary'}>{tCommon(`status.${status.toLowerCase()}`)}</Badge>
  }

  return (
    <div className="p-6">
      <div className="mb-6 flex items-center justify-between">
        <h1 className="text-3xl font-bold">{t('title')}</h1>
        <Button onClick={() => router.push('/admin/users/new')}>
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
                    <TableHead>{t('fields.name')}</TableHead>
                    <TableHead>{t('fields.email')}</TableHead>
                    <TableHead>{t('fields.role')}</TableHead>
                    <TableHead>{t('fields.status')}</TableHead>
                    <TableHead>{t('fields.createdAt')}</TableHead>
                    <TableHead className="w-[70px]">{tCommon('table.actions')}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {data?.data?.users?.map((user: User) => (
                    <TableRow key={user.id}>
                      <TableCell>
                        <div className="flex items-center gap-3">
                          <Avatar className="h-8 w-8">
                            <AvatarImage src={user.avatar} />
                            <AvatarFallback>{getInitials(user.name)}</AvatarFallback>
                          </Avatar>
                          <span className="font-medium">{user.name}</span>
                        </div>
                      </TableCell>
                      <TableCell>{user.email}</TableCell>
                      <TableCell>
                        <Badge variant="outline">{user.role.name}</Badge>
                      </TableCell>
                      <TableCell>{getStatusBadge(user.status)}</TableCell>
                      <TableCell>{new Date(user.createdAt).toLocaleDateString()}</TableCell>
                      <TableCell>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="icon">
                              <MoreHorizontal className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem onClick={() => router.push(`/admin/users/${user.id}`)}>
                              <Pencil className="mr-2 h-4 w-4" />
                              {tCommon('actions.edit')}
                            </DropdownMenuItem>
                            <DropdownMenuItem className="text-destructive">
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
                    {tCommon('pagination.showing')} {data.data.users.length} / {data.data.pagination.total} {tCommon('pagination.items')}
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
