'use client'

import { useState, useEffect } from 'react'
import { useTranslations } from 'next-intl'
import { useRouter } from 'next/navigation'
import { nanoid } from 'nanoid'
import { useChatStore, type ChatSession } from '@/stores/chat-store'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Plus, MessageSquare, Trash2 } from 'lucide-react'
import { ChatWindow } from '@/components/chat/chat-window'
import { cn } from '@/lib/utils'

export default function ChatPage() {
  const t = useTranslations('dashboard.chat')
  const router = useRouter()
  const { sessions, activeSessionId, addSession, removeSession, setActiveSession } = useChatStore()
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    setMounted(true)
  }, [])

  const handleNewChat = () => {
    const newSession: ChatSession = {
      id: nanoid(),
      title: 'New Chat',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    }
    addSession(newSession)
    setActiveSession(newSession.id)
  }

  const handleDeleteSession = (sessionId: string, e: React.MouseEvent) => {
    e.stopPropagation()
    removeSession(sessionId)
  }

  if (!mounted) {
    return (
      <div className="flex h-[calc(100vh-4rem)] items-center justify-center">
        <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
      </div>
    )
  }

  return (
    <div className="flex h-[calc(100vh-4rem)]">
      {/* Sidebar */}
      <div className="flex w-64 flex-col border-r bg-muted/30">
        <div className="p-4">
          <Button onClick={handleNewChat} className="w-full gap-2">
            <Plus className="h-4 w-4" />
            {t('newChat')}
          </Button>
        </div>

        <ScrollArea className="flex-1">
          <div className="space-y-1 p-2">
            {sessions.length === 0 ? (
              <p className="px-3 py-8 text-center text-sm text-muted-foreground">
                No chat history
              </p>
            ) : (
              sessions.map((session) => (
                <div
                  key={session.id}
                  onClick={() => setActiveSession(session.id)}
                  className={cn(
                    'group flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors hover:bg-accent',
                    activeSessionId === session.id && 'bg-accent'
                  )}
                >
                  <MessageSquare className="h-4 w-4 shrink-0" />
                  <span className="flex-1 truncate">{session.title}</span>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6 opacity-0 group-hover:opacity-100"
                    onClick={(e) => handleDeleteSession(session.id, e)}
                  >
                    <Trash2 className="h-3 w-3" />
                  </Button>
                </div>
              ))
            )}
          </div>
        </ScrollArea>
      </div>

      {/* Chat Window */}
      <div className="flex-1">
        {activeSessionId ? (
          <ChatWindow sessionId={activeSessionId} />
        ) : (
          <div className="flex h-full flex-col items-center justify-center gap-4 text-muted-foreground">
            <MessageSquare className="h-12 w-12" />
            <p>{t('placeholder')}</p>
            <Button onClick={handleNewChat} variant="outline">
              {t('newChat')}
            </Button>
          </div>
        )}
      </div>
    </div>
  )
}
