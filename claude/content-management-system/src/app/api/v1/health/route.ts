import { NextResponse } from 'next/server'
import { db } from '@/lib/db'

export const dynamic = 'force-dynamic'

export async function GET() {
  let databaseStatus: 'connected' | 'disconnected' = 'disconnected'
  
  try {
    await db.$queryRaw`SELECT 1`
    databaseStatus = 'connected'
  } catch {
    databaseStatus = 'disconnected'
  }

  return NextResponse.json({
    status: databaseStatus === 'connected' ? 'healthy' : 'unhealthy',
    timestamp: new Date().toISOString(),
    database: databaseStatus,
    version: '1.0.0',
  }, { 
    status: databaseStatus === 'connected' ? 200 : 503,
    headers: { 'Cache-Control': 'no-store' },
  })
}
