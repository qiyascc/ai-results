import bcrypt from 'bcryptjs'

const SALT_ROUNDS = 12

export async function hashPassword(password: string): Promise<string> {
  return bcrypt.hash(password, SALT_ROUNDS)
}

export async function verifyPassword(password: string, hash: string): Promise<boolean> {
  return bcrypt.compare(password, hash)
}

export function checkPasswordStrength(password: string): {
  valid: boolean
  errors: string[]
  score: number
} {
  const errors: string[] = []
  let score = 0
  
  if (password.length < 8) {
    errors.push('Password must be at least 8 characters')
  } else {
    score += 1
  }
  
  if (!/[a-z]/.test(password)) {
    errors.push('Password must contain a lowercase letter')
  } else {
    score += 1
  }
  
  if (!/[A-Z]/.test(password)) {
    errors.push('Password must contain an uppercase letter')
  } else {
    score += 1
  }
  
  if (!/\d/.test(password)) {
    errors.push('Password must contain a number')
  } else {
    score += 1
  }
  
  if (!/[!@#$%^&*(),.?":{}|<>]/.test(password)) {
    errors.push('Password must contain a special character')
  } else {
    score += 1
  }
  
  return { valid: errors.length === 0, errors, score: Math.min(score, 5) }
}
