import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import * as crypto from '../crypto/worker';

interface Identity {
  publicKey: string;
  fingerprint: string;
}

interface AuthState {
  isAuthenticated: boolean;
  identity: Identity | null;
  deviceId: string | null;
  isInitialized: boolean;
  
  initialize: () => Promise<void>;
  createIdentity: () => Promise<void>;
  loadIdentity: () => Promise<boolean>;
  logout: () => void;
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      isAuthenticated: false,
      identity: null,
      deviceId: null,
      isInitialized: false,

      initialize: async () => {
        await crypto.init();
        const loaded = await get().loadIdentity();
        
        if (!loaded) {
          set({ isInitialized: true, isAuthenticated: false });
        }
      },

      createIdentity: async () => {
        await crypto.init();
        const keyPair = crypto.generateIdentityKeyPair();
        const fingerprint = crypto.generateFingerprint(keyPair.signing.publicKey);
        const deviceId = crypto.toBase64(crypto.generateEphemeralKeyPair().publicKey).slice(0, 16);
        
        // Store identity securely (in production, encrypt with user password)
        const identity = {
          publicKey: crypto.toBase64(keyPair.signing.publicKey),
          fingerprint,
        };
        
        // Store secret key in IndexedDB (encrypted in production)
        const { openDB } = await import('idb');
        const db = await openDB('qiyashash', 1, {
          upgrade(db) {
            db.createObjectStore('keys');
            db.createObjectStore('sessions');
            db.createObjectStore('messages');
          },
        });
        
        await db.put('keys', {
          signing: {
            public: crypto.toBase64(keyPair.signing.publicKey),
            secret: crypto.toBase64(keyPair.signing.secretKey),
          },
          exchange: {
            public: crypto.toBase64(keyPair.exchange.publicKey),
            secret: crypto.toBase64(keyPair.exchange.secretKey),
          },
        }, 'identity');
        
        set({
          isAuthenticated: true,
          identity,
          deviceId,
          isInitialized: true,
        });
      },

      loadIdentity: async () => {
        try {
          const { openDB } = await import('idb');
          const db = await openDB('qiyashash', 1, {
            upgrade(db) {
              db.createObjectStore('keys');
              db.createObjectStore('sessions');
              db.createObjectStore('messages');
            },
          });
          
          const storedKeys = await db.get('keys', 'identity');
          
          if (storedKeys) {
            const publicKey = storedKeys.signing.public;
            const fingerprint = crypto.generateFingerprint(crypto.fromBase64(publicKey));
            
            set({
              isAuthenticated: true,
              identity: { publicKey, fingerprint },
              isInitialized: true,
            });
            return true;
          }
          return false;
        } catch {
          return false;
        }
      },

      logout: () => {
        // Clear all data
        indexedDB.deleteDatabase('qiyashash');
        set({
          isAuthenticated: false,
          identity: null,
          deviceId: null,
        });
      },
    }),
    {
      name: 'qiyashash-auth',
      partialize: (state) => ({
        deviceId: state.deviceId,
      }),
    }
  )
);
