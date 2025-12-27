import { Server } from 'socket.io'
import { createServer } from 'http'
import { Redis } from 'ioredis'
import { createAdapter } from '@socket.io/redis-adapter'

const PORT = parseInt(process.env.SOCKET_PORT || '3001', 10)
const REDIS_URL = process.env.REDIS_URL || 'redis://localhost:6379'
const CORS_ORIGIN = process.env.NEXT_PUBLIC_APP_URL || 'http://localhost:3000'

const pubClient = new Redis(REDIS_URL)
const subClient = pubClient.duplicate()

const httpServer = createServer()

const io = new Server(httpServer, {
  cors: { origin: CORS_ORIGIN, methods: ['GET', 'POST'], credentials: true },
  transports: ['websocket', 'polling'],
})

io.adapter(createAdapter(pubClient, subClient))

io.use(async (socket, next) => {
  const userId = socket.handshake.auth.userId
  if (!userId) return next(new Error('Authentication required'))
  socket.data.userId = userId
  next()
})

io.on('connection', async (socket) => {
  const { userId } = socket.data
  console.log(`[Socket] User ${userId} connected`)

  socket.join(`user:${userId}`)
  await pubClient.hset(`presence:${userId}`, { status: 'online', socketId: socket.id, lastSeen: new Date().toISOString() })
  socket.broadcast.emit('presence:update', { userId, status: 'online', lastSeen: new Date().toISOString() })

  socket.on('chat:join', (sessionId) => {
    socket.join(`chat:${sessionId}`)
  })

  socket.on('chat:leave', (sessionId) => {
    socket.leave(`chat:${sessionId}`)
  })

  socket.on('chat:message', async (data) => {
    const { sessionId, content } = data
    const userMessage = { id: `msg_${Date.now()}`, sessionId, role: 'user', content, timestamp: new Date().toISOString() }
    io.to(`chat:${sessionId}`).emit('chat:message', userMessage)
    io.to(`chat:${sessionId}`).emit('chat:typing', { sessionId, userId: 'ai', isAI: true })

    // Simulate AI response
    setTimeout(() => {
      io.to(`chat:${sessionId}`).emit('chat:stop-typing', { sessionId, userId: 'ai' })
      io.to(`chat:${sessionId}`).emit('chat:message', {
        id: `msg_${Date.now()}`,
        sessionId,
        role: 'assistant',
        content: `AI response to: "${content}"`,
        timestamp: new Date().toISOString(),
      })
    }, 1000)
  })

  socket.on('chat:typing', (sessionId) => {
    socket.to(`chat:${sessionId}`).emit('chat:typing', { sessionId, userId })
  })

  socket.on('chat:stop-typing', (sessionId) => {
    socket.to(`chat:${sessionId}`).emit('chat:stop-typing', { sessionId, userId })
  })

  socket.on('disconnect', async () => {
    console.log(`[Socket] User ${userId} disconnected`)
    await pubClient.hset(`presence:${userId}`, { status: 'offline', lastSeen: new Date().toISOString() })
    socket.broadcast.emit('presence:update', { userId, status: 'offline', lastSeen: new Date().toISOString() })
  })
})

const shutdown = async () => {
  io.close()
  await pubClient.quit()
  await subClient.quit()
  process.exit(0)
}

process.on('SIGTERM', shutdown)
process.on('SIGINT', shutdown)

httpServer.listen(PORT, () => console.log(`🔌 Socket server running on port ${PORT}`))
