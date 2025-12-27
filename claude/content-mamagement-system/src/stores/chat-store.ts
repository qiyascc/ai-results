import { create } from 'zustand'
import { devtools, persist } from 'zustand/middleware'

export interface Message {
  id: string
  sessionId: string
  role: 'user' | 'assistant' | 'system'
  content: string
  timestamp: string
  metadata?: Record<string, unknown>
}

export interface ChatSession {
  id: string
  title: string
  createdAt: string
  updatedAt: string
  lastMessage?: string
}

interface ChatState {
  sessions: ChatSession[]
  activeSessionId: string | null
  messages: Record<string, Message[]>
  isTyping: Record<string, boolean>
  
  // Actions
  addSession: (session: ChatSession) => void
  removeSession: (sessionId: string) => void
  setActiveSession: (sessionId: string) => void
  addMessage: (message: Message) => void
  setMessages: (sessionId: string, messages: Message[]) => void
  clearMessages: (sessionId: string) => void
  setTyping: (userId: string, isTyping: boolean) => void
  updateSessionTitle: (sessionId: string, title: string) => void
}

export const useChatStore = create<ChatState>()(
  devtools(
    persist(
      (set, get) => ({
        sessions: [],
        activeSessionId: null,
        messages: {},
        isTyping: {},

        addSession: (session) =>
          set((state) => ({
            sessions: [session, ...state.sessions],
          })),

        removeSession: (sessionId) =>
          set((state) => {
            const { [sessionId]: _, ...remainingMessages } = state.messages
            return {
              sessions: state.sessions.filter((s) => s.id !== sessionId),
              messages: remainingMessages,
              activeSessionId: state.activeSessionId === sessionId ? null : state.activeSessionId,
            }
          }),

        setActiveSession: (sessionId) =>
          set({ activeSessionId: sessionId }),

        addMessage: (message) =>
          set((state) => {
            const sessionMessages = state.messages[message.sessionId] || []
            return {
              messages: {
                ...state.messages,
                [message.sessionId]: [...sessionMessages, message],
              },
              sessions: state.sessions.map((s) =>
                s.id === message.sessionId
                  ? { ...s, updatedAt: message.timestamp, lastMessage: message.content }
                  : s
              ),
            }
          }),

        setMessages: (sessionId, messages) =>
          set((state) => ({
            messages: {
              ...state.messages,
              [sessionId]: messages,
            },
          })),

        clearMessages: (sessionId) =>
          set((state) => ({
            messages: {
              ...state.messages,
              [sessionId]: [],
            },
          })),

        setTyping: (userId, isTyping) =>
          set((state) => ({
            isTyping: {
              ...state.isTyping,
              [userId]: isTyping,
            },
          })),

        updateSessionTitle: (sessionId, title) =>
          set((state) => ({
            sessions: state.sessions.map((s) =>
              s.id === sessionId ? { ...s, title } : s
            ),
          })),
      }),
      {
        name: 'qiyas-chat-store',
        partialize: (state) => ({
          sessions: state.sessions.slice(0, 50), // Keep last 50 sessions
        }),
      }
    ),
    { name: 'ChatStore' }
  )
)
