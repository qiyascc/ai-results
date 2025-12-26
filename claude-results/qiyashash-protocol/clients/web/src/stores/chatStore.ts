import { create } from 'zustand';

interface Message {
  id: string;
  senderId: string;
  recipientId: string;
  content: string;
  timestamp: number;
  status: 'pending' | 'sent' | 'delivered' | 'read' | 'failed';
}

interface Conversation {
  id: string;
  participantId: string;
  participantName?: string;
  lastMessage?: Message;
  unreadCount: number;
  isVerified: boolean;
}

interface ChatState {
  conversations: Conversation[];
  messages: Record<string, Message[]>;
  activeConversation: string | null;
  isLoading: boolean;
  
  setActiveConversation: (id: string | null) => void;
  addMessage: (message: Message) => void;
  updateMessageStatus: (messageId: string, status: Message['status']) => void;
  loadConversations: () => Promise<void>;
  loadMessages: (conversationId: string) => Promise<void>;
  sendMessage: (recipientId: string, content: string) => Promise<void>;
  markAsRead: (conversationId: string) => void;
}

export const useChatStore = create<ChatState>((set, get) => ({
  conversations: [],
  messages: {},
  activeConversation: null,
  isLoading: false,

  setActiveConversation: (id) => {
    set({ activeConversation: id });
    if (id) {
      get().markAsRead(id);
    }
  },

  addMessage: (message) => {
    set((state) => {
      const conversationId = message.senderId === 'me' 
        ? message.recipientId 
        : message.senderId;
      
      const existingMessages = state.messages[conversationId] || [];
      
      return {
        messages: {
          ...state.messages,
          [conversationId]: [...existingMessages, message],
        },
        conversations: state.conversations.map((conv) =>
          conv.participantId === conversationId
            ? { ...conv, lastMessage: message }
            : conv
        ),
      };
    });
  },

  updateMessageStatus: (messageId, status) => {
    set((state) => {
      const newMessages = { ...state.messages };
      
      for (const convId of Object.keys(newMessages)) {
        newMessages[convId] = newMessages[convId].map((msg) =>
          msg.id === messageId ? { ...msg, status } : msg
        );
      }
      
      return { messages: newMessages };
    });
  },

  loadConversations: async () => {
    set({ isLoading: true });
    
    try {
      // Load from IndexedDB
      const { openDB } = await import('idb');
      const db = await openDB('qiyashash', 1);
      
      // In production, load actual conversations
      // For now, return empty array
      set({ conversations: [], isLoading: false });
    } catch (error) {
      console.error('Failed to load conversations:', error);
      set({ isLoading: false });
    }
  },

  loadMessages: async (conversationId) => {
    set({ isLoading: true });
    
    try {
      const { openDB } = await import('idb');
      const db = await openDB('qiyashash', 1);
      
      const tx = db.transaction('messages', 'readonly');
      const store = tx.objectStore('messages');
      const allMessages = await store.getAll();
      
      const convMessages = allMessages.filter(
        (msg: Message) =>
          msg.senderId === conversationId || msg.recipientId === conversationId
      );
      
      convMessages.sort((a: Message, b: Message) => a.timestamp - b.timestamp);
      
      set((state) => ({
        messages: {
          ...state.messages,
          [conversationId]: convMessages,
        },
        isLoading: false,
      }));
    } catch (error) {
      console.error('Failed to load messages:', error);
      set({ isLoading: false });
    }
  },

  sendMessage: async (recipientId, content) => {
    const message: Message = {
      id: crypto.randomUUID(),
      senderId: 'me',
      recipientId,
      content,
      timestamp: Date.now(),
      status: 'pending',
    };
    
    get().addMessage(message);
    
    try {
      // Encrypt and send via relay/DHT
      // In production, use actual encryption and network
      
      // Store locally
      const { openDB } = await import('idb');
      const db = await openDB('qiyashash', 1);
      await db.put('messages', message, message.id);
      
      // Update status
      get().updateMessageStatus(message.id, 'sent');
    } catch (error) {
      console.error('Failed to send message:', error);
      get().updateMessageStatus(message.id, 'failed');
    }
  },

  markAsRead: (conversationId) => {
    set((state) => ({
      conversations: state.conversations.map((conv) =>
        conv.participantId === conversationId
          ? { ...conv, unreadCount: 0 }
          : conv
      ),
    }));
  },
}));
