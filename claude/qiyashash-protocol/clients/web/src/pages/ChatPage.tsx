import { useState, useEffect, useRef } from 'react';
import { useParams } from 'react-router-dom';
import { Send, Shield, CheckCheck, Check, Clock } from 'lucide-react';
import { useChatStore } from '../stores/chatStore';
import { useAuthStore } from '../stores/authStore';
import { cn } from '../lib/utils';
import { formatDistanceToNow } from 'date-fns';

export function ChatPage() {
  const { userId } = useParams();
  const [newMessage, setNewMessage] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);
  
  const {
    conversations,
    messages,
    activeConversation,
    setActiveConversation,
    loadConversations,
    loadMessages,
    sendMessage,
  } = useChatStore();
  
  const { identity } = useAuthStore();

  useEffect(() => {
    loadConversations();
  }, [loadConversations]);

  useEffect(() => {
    if (userId) {
      setActiveConversation(userId);
      loadMessages(userId);
    }
  }, [userId, setActiveConversation, loadMessages]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, activeConversation]);

  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newMessage.trim() || !activeConversation) return;
    
    await sendMessage(activeConversation, newMessage.trim());
    setNewMessage('');
  };

  const currentMessages = activeConversation ? messages[activeConversation] || [] : [];

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'pending':
        return <Clock className="w-3 h-3 text-gray-500" />;
      case 'sent':
        return <Check className="w-3 h-3 text-gray-500" />;
      case 'delivered':
        return <CheckCheck className="w-3 h-3 text-gray-500" />;
      case 'read':
        return <CheckCheck className="w-3 h-3 text-emerald-400" />;
      default:
        return null;
    }
  };

  return (
    <div className="flex h-full">
      {/* Conversations list */}
      <div className="w-80 bg-gray-800 border-r border-gray-700 flex flex-col">
        <div className="p-4 border-b border-gray-700">
          <h2 className="text-lg font-semibold">Conversations</h2>
        </div>
        
        <div className="flex-1 overflow-y-auto">
          {conversations.length === 0 ? (
            <div className="p-4 text-center text-gray-500">
              <p>No conversations yet</p>
              <p className="text-sm mt-2">
                Share your fingerprint to start chatting
              </p>
            </div>
          ) : (
            conversations.map((conv) => (
              <button
                key={conv.id}
                onClick={() => setActiveConversation(conv.participantId)}
                className={cn(
                  'w-full p-4 text-left border-b border-gray-700 hover:bg-gray-700 transition-colors',
                  activeConversation === conv.participantId && 'bg-gray-700'
                )}
              >
                <div className="flex items-center justify-between">
                  <span className="font-medium">
                    {conv.participantName || conv.participantId.slice(0, 8)}...
                  </span>
                  {conv.isVerified && (
                    <Shield className="w-4 h-4 text-emerald-400" />
                  )}
                </div>
                {conv.lastMessage && (
                  <p className="text-sm text-gray-400 truncate mt-1">
                    {conv.lastMessage.content}
                  </p>
                )}
                {conv.unreadCount > 0 && (
                  <span className="inline-flex items-center justify-center w-5 h-5 text-xs bg-emerald-600 rounded-full">
                    {conv.unreadCount}
                  </span>
                )}
              </button>
            ))
          )}
        </div>
      </div>

      {/* Chat area */}
      <div className="flex-1 flex flex-col">
        {activeConversation ? (
          <>
            {/* Chat header */}
            <div className="p-4 bg-gray-800 border-b border-gray-700 flex items-center justify-between">
              <div>
                <h3 className="font-medium">
                  {activeConversation.slice(0, 16)}...
                </h3>
                <p className="text-xs text-emerald-400 flex items-center gap-1">
                  <Shield className="w-3 h-3" />
                  End-to-end encrypted
                </p>
              </div>
            </div>

            {/* Messages */}
            <div className="flex-1 overflow-y-auto p-4 space-y-4">
              {currentMessages.map((msg) => (
                <div
                  key={msg.id}
                  className={cn(
                    'flex',
                    msg.senderId === 'me' ? 'justify-end' : 'justify-start'
                  )}
                >
                  <div
                    className={cn(
                      'max-w-md px-4 py-2 rounded-2xl',
                      msg.senderId === 'me'
                        ? 'bg-emerald-600 text-white rounded-br-none'
                        : 'bg-gray-700 text-white rounded-bl-none'
                    )}
                  >
                    <p>{msg.content}</p>
                    <div className="flex items-center justify-end gap-1 mt-1">
                      <span className="text-xs opacity-70">
                        {formatDistanceToNow(msg.timestamp, { addSuffix: true })}
                      </span>
                      {msg.senderId === 'me' && getStatusIcon(msg.status)}
                    </div>
                  </div>
                </div>
              ))}
              <div ref={messagesEndRef} />
            </div>

            {/* Message input */}
            <form onSubmit={handleSend} className="p-4 bg-gray-800 border-t border-gray-700">
              <div className="flex gap-2">
                <input
                  type="text"
                  value={newMessage}
                  onChange={(e) => setNewMessage(e.target.value)}
                  placeholder="Type a message..."
                  className="flex-1 px-4 py-2 bg-gray-700 border border-gray-600 rounded-full text-white placeholder-gray-400 focus:outline-none focus:border-emerald-500"
                />
                <button
                  type="submit"
                  disabled={!newMessage.trim()}
                  className="px-4 py-2 bg-emerald-600 rounded-full text-white hover:bg-emerald-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                >
                  <Send className="w-5 h-5" />
                </button>
              </div>
            </form>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center text-gray-500">
            <div className="text-center">
              <Shield className="w-16 h-16 mx-auto mb-4 text-gray-600" />
              <h3 className="text-xl font-medium mb-2">QiyasHash E2E Chat</h3>
              <p>Select a conversation or start a new one</p>
              <div className="mt-4 p-4 bg-gray-800 rounded-lg">
                <p className="text-sm text-gray-400 mb-2">Your Fingerprint:</p>
                <code className="text-xs text-emerald-400 font-mono block break-all">
                  {identity?.fingerprint}
                </code>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
