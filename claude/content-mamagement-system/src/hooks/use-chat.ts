'use client'

import { useState, useCallback, useEffect, useRef } from 'react'
import { useSocket } from './use-socket'
import { useChatStore, type Message } from '@/stores/chat-store'

export function useChat(sessionId: string) {
  const { emit, on, off } = useSocket()
  const {
    messages,
    isTyping,
    addMessage,
    setTyping,
    clearMessages,
    setActiveSession,
  } = useChatStore()
  
  const [isLoading, setIsLoading] = useState(false)
  const messagesEndRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!sessionId) return

    setActiveSession(sessionId)
    emit('chat:join', sessionId)

    const handleMessage = (message: Message) => {
      addMessage(message)
      setIsLoading(false)
    }

    const handleTyping = (data: { sessionId: string; userId: string; isAI?: boolean }) => {
      if (data.sessionId === sessionId) {
        setTyping(data.isAI ? 'ai' : data.userId, true)
      }
    }

    const handleStopTyping = (data: { sessionId: string; userId: string }) => {
      if (data.sessionId === sessionId) {
        setTyping(data.userId, false)
      }
    }

    const unsubMessage = on<Message>('chat:message', handleMessage)
    const unsubTyping = on<{ sessionId: string; userId: string; isAI?: boolean }>('chat:typing', handleTyping)
    const unsubStopTyping = on<{ sessionId: string; userId: string }>('chat:stop-typing', handleStopTyping)

    return () => {
      emit('chat:leave', sessionId)
      unsubMessage()
      unsubTyping()
      unsubStopTyping()
    }
  }, [sessionId, emit, on, addMessage, setTyping, setActiveSession])

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  const sendMessage = useCallback(
    (content: string) => {
      if (!content.trim() || !sessionId) return

      setIsLoading(true)
      emit('chat:message', { sessionId, content: content.trim() })
    },
    [sessionId, emit]
  )

  const startTyping = useCallback(() => {
    emit('chat:typing', sessionId)
  }, [sessionId, emit])

  const stopTyping = useCallback(() => {
    emit('chat:stop-typing', sessionId)
  }, [sessionId, emit])

  return {
    messages: messages[sessionId] || [],
    isTyping,
    isLoading,
    sendMessage,
    startTyping,
    stopTyping,
    clearMessages: () => clearMessages(sessionId),
    messagesEndRef,
  }
}
