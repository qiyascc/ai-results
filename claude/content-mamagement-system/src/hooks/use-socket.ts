'use client'

import { useEffect, useRef, useCallback } from 'react'
import { io, Socket } from 'socket.io-client'
import { useAuthStore } from '@/stores/auth-store'

const SOCKET_URL = process.env.NEXT_PUBLIC_SOCKET_URL || 'http://localhost:3001'

export function useSocket() {
  const socketRef = useRef<Socket | null>(null)
  const { user, isAuthenticated } = useAuthStore()

  useEffect(() => {
    if (!isAuthenticated || !user?.id) return

    socketRef.current = io(SOCKET_URL, {
      auth: { userId: user.id },
      transports: ['websocket', 'polling'],
      reconnection: true,
      reconnectionAttempts: 5,
      reconnectionDelay: 1000,
    })

    socketRef.current.on('connect', () => {
      console.log('[Socket] Connected')
    })

    socketRef.current.on('disconnect', (reason) => {
      console.log('[Socket] Disconnected:', reason)
    })

    socketRef.current.on('connect_error', (error) => {
      console.error('[Socket] Connection error:', error.message)
    })

    return () => {
      socketRef.current?.disconnect()
      socketRef.current = null
    }
  }, [isAuthenticated, user?.id])

  const emit = useCallback(<T>(event: string, data?: T) => {
    socketRef.current?.emit(event, data)
  }, [])

  const on = useCallback(<T>(event: string, callback: (data: T) => void) => {
    socketRef.current?.on(event, callback)
    return () => {
      socketRef.current?.off(event, callback)
    }
  }, [])

  const off = useCallback((event: string) => {
    socketRef.current?.off(event)
  }, [])

  const joinRoom = useCallback((room: string) => {
    socketRef.current?.emit('room:join', room)
  }, [])

  const leaveRoom = useCallback((room: string) => {
    socketRef.current?.emit('room:leave', room)
  }, [])

  return {
    socket: socketRef.current,
    isConnected: socketRef.current?.connected ?? false,
    emit,
    on,
    off,
    joinRoom,
    leaveRoom,
  }
}
