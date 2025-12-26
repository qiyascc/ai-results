/**
 * QiyasHash Crypto Worker
 * 
 * Offloads cryptographic operations to a web worker to keep UI responsive.
 * Uses libsodium for all cryptographic primitives.
 */

import _sodium from 'libsodium-wrappers-sumo';

let sodium: typeof _sodium;

// Types
interface KeyPair {
  publicKey: Uint8Array;
  secretKey: Uint8Array;
}

interface IdentityKeyPair {
  signing: KeyPair;
  exchange: KeyPair;
}

interface PreKeyBundle {
  identityKey: Uint8Array;
  signedPreKey: Uint8Array;
  signedPreKeyId: number;
  signedPreKeySignature: Uint8Array;
  oneTimePreKey?: Uint8Array;
  oneTimePreKeyId?: number;
}

interface RatchetState {
  rootKey: Uint8Array;
  sendingChainKey: Uint8Array;
  receivingChainKey: Uint8Array;
  sendingRatchetKey: KeyPair;
  receivingRatchetKey?: Uint8Array;
  sendingCounter: number;
  receivingCounter: number;
  previousCounter: number;
  skippedKeys: Map<string, Uint8Array>;
}

interface EncryptedMessage {
  header: {
    dhPublic: Uint8Array;
    messageNumber: number;
    previousChainLength: number;
  };
  ciphertext: Uint8Array;
  nonce: Uint8Array;
}

// Initialize sodium
async function init(): Promise<void> {
  await _sodium.ready;
  sodium = _sodium;
}

// Identity Key Generation
function generateIdentityKeyPair(): IdentityKeyPair {
  const signing = sodium.crypto_sign_keypair();
  
  // Derive X25519 key from Ed25519 signing key
  const exchangeSecret = sodium.crypto_sign_ed25519_sk_to_curve25519(signing.privateKey);
  const exchangePublic = sodium.crypto_sign_ed25519_pk_to_curve25519(signing.publicKey);
  
  return {
    signing: {
      publicKey: signing.publicKey,
      secretKey: signing.privateKey,
    },
    exchange: {
      publicKey: exchangePublic,
      secretKey: exchangeSecret,
    },
  };
}

// Generate ephemeral X25519 key pair
function generateEphemeralKeyPair(): KeyPair {
  const keyPair = sodium.crypto_kx_keypair();
  return {
    publicKey: keyPair.publicKey,
    secretKey: keyPair.privateKey,
  };
}

// Generate signed pre-key
function generateSignedPreKey(identityKey: KeyPair): {
  keyPair: KeyPair;
  signature: Uint8Array;
} {
  const keyPair = generateEphemeralKeyPair();
  const signature = sodium.crypto_sign_detached(keyPair.publicKey, identityKey.secretKey);
  
  return { keyPair, signature };
}

// X3DH Key Agreement (Initiator)
function x3dhInitiate(
  ourIdentity: IdentityKeyPair,
  theirBundle: PreKeyBundle
): { sharedSecret: Uint8Array; ephemeralPublic: Uint8Array } {
  // Generate ephemeral key
  const ephemeral = generateEphemeralKeyPair();
  
  // Verify signed pre-key signature
  if (!sodium.crypto_sign_verify_detached(
    theirBundle.signedPreKeySignature,
    theirBundle.signedPreKey,
    theirBundle.identityKey
  )) {
    throw new Error('Invalid signed pre-key signature');
  }
  
  // Convert their identity key to X25519
  const theirIdentityX25519 = sodium.crypto_sign_ed25519_pk_to_curve25519(theirBundle.identityKey);
  
  // DH1 = DH(IK_A, SPK_B)
  const dh1 = sodium.crypto_scalarmult(ourIdentity.exchange.secretKey, theirBundle.signedPreKey);
  
  // DH2 = DH(EK_A, IK_B)
  const dh2 = sodium.crypto_scalarmult(ephemeral.secretKey, theirIdentityX25519);
  
  // DH3 = DH(EK_A, SPK_B)
  const dh3 = sodium.crypto_scalarmult(ephemeral.secretKey, theirBundle.signedPreKey);
  
  // DH4 = DH(EK_A, OPK_B) if available
  let dh4 = new Uint8Array(32);
  if (theirBundle.oneTimePreKey) {
    dh4 = sodium.crypto_scalarmult(ephemeral.secretKey, theirBundle.oneTimePreKey);
  }
  
  // Combine DH outputs
  const combined = new Uint8Array(dh1.length + dh2.length + dh3.length + dh4.length);
  combined.set(dh1, 0);
  combined.set(dh2, dh1.length);
  combined.set(dh3, dh1.length + dh2.length);
  combined.set(dh4, dh1.length + dh2.length + dh3.length);
  
  // Derive shared secret using HKDF
  const sharedSecret = hkdfDerive(combined, new Uint8Array(0), 'QiyasHash_X3DH_v1', 32);
  
  return { sharedSecret, ephemeralPublic: ephemeral.publicKey };
}

// HKDF key derivation
function hkdfDerive(
  inputKey: Uint8Array,
  salt: Uint8Array,
  info: string,
  length: number
): Uint8Array {
  // Use HKDF-SHA512
  const infoBytes = sodium.from_string(info);
  
  // Extract
  const prk = sodium.crypto_generichash(64, inputKey, salt.length > 0 ? salt : undefined);
  
  // Expand
  const result = new Uint8Array(length);
  let counter = 1;
  let prev = new Uint8Array(0);
  let offset = 0;
  
  while (offset < length) {
    const data = new Uint8Array(prev.length + infoBytes.length + 1);
    data.set(prev, 0);
    data.set(infoBytes, prev.length);
    data[prev.length + infoBytes.length] = counter;
    
    prev = sodium.crypto_generichash(64, data, prk);
    const toCopy = Math.min(prev.length, length - offset);
    result.set(prev.subarray(0, toCopy), offset);
    offset += toCopy;
    counter++;
  }
  
  return result;
}

// Double Ratchet - Initialize as initiator
function ratchetInit(sharedSecret: Uint8Array, theirRatchetKey: Uint8Array): RatchetState {
  const ratchetKeyPair = generateEphemeralKeyPair();
  const dh = sodium.crypto_scalarmult(ratchetKeyPair.secretKey, theirRatchetKey);
  
  const keys = hkdfDerive(dh, sharedSecret, 'QiyasHash_Ratchet_v1', 64);
  
  return {
    rootKey: keys.subarray(0, 32),
    sendingChainKey: keys.subarray(32, 64),
    receivingChainKey: new Uint8Array(32),
    sendingRatchetKey: ratchetKeyPair,
    receivingRatchetKey: theirRatchetKey,
    sendingCounter: 0,
    receivingCounter: 0,
    previousCounter: 0,
    skippedKeys: new Map(),
  };
}

// Derive message key from chain key
function deriveMessageKey(chainKey: Uint8Array): { newChainKey: Uint8Array; messageKey: Uint8Array } {
  const newChainKey = sodium.crypto_generichash(32, chainKey, sodium.from_string('chain'));
  const messageKey = sodium.crypto_generichash(32, chainKey, sodium.from_string('message'));
  
  return { newChainKey, messageKey };
}

// Encrypt message
function ratchetEncrypt(state: RatchetState, plaintext: Uint8Array): { state: RatchetState; message: EncryptedMessage } {
  const { newChainKey, messageKey } = deriveMessageKey(state.sendingChainKey);
  
  const nonce = sodium.randombytes_buf(24);
  const ciphertext = sodium.crypto_aead_xchacha20poly1305_ietf_encrypt(
    plaintext,
    null,
    null,
    nonce,
    messageKey
  );
  
  const message: EncryptedMessage = {
    header: {
      dhPublic: state.sendingRatchetKey.publicKey,
      messageNumber: state.sendingCounter,
      previousChainLength: state.previousCounter,
    },
    ciphertext,
    nonce,
  };
  
  return {
    state: {
      ...state,
      sendingChainKey: newChainKey,
      sendingCounter: state.sendingCounter + 1,
    },
    message,
  };
}

// Decrypt message
function ratchetDecrypt(state: RatchetState, message: EncryptedMessage): { state: RatchetState; plaintext: Uint8Array } {
  // Check if we need to perform DH ratchet
  let newState = state;
  
  if (!state.receivingRatchetKey || 
      !sodium.memcmp(message.header.dhPublic, state.receivingRatchetKey)) {
    // DH Ratchet step
    const dh1 = sodium.crypto_scalarmult(state.sendingRatchetKey.secretKey, message.header.dhPublic);
    const keys1 = hkdfDerive(dh1, state.rootKey, 'QiyasHash_Ratchet_v1', 64);
    
    const newRatchetKey = generateEphemeralKeyPair();
    const dh2 = sodium.crypto_scalarmult(newRatchetKey.secretKey, message.header.dhPublic);
    const keys2 = hkdfDerive(dh2, keys1.subarray(0, 32), 'QiyasHash_Ratchet_v1', 64);
    
    newState = {
      ...state,
      rootKey: keys2.subarray(0, 32),
      sendingChainKey: keys2.subarray(32, 64),
      receivingChainKey: keys1.subarray(32, 64),
      sendingRatchetKey: newRatchetKey,
      receivingRatchetKey: message.header.dhPublic,
      previousCounter: state.sendingCounter,
      sendingCounter: 0,
      receivingCounter: 0,
    };
  }
  
  // Derive message key
  const { newChainKey, messageKey } = deriveMessageKey(newState.receivingChainKey);
  
  // Decrypt
  const plaintext = sodium.crypto_aead_xchacha20poly1305_ietf_decrypt(
    null,
    message.ciphertext,
    null,
    message.nonce,
    messageKey
  );
  
  if (!plaintext) {
    throw new Error('Decryption failed');
  }
  
  return {
    state: {
      ...newState,
      receivingChainKey: newChainKey,
      receivingCounter: newState.receivingCounter + 1,
    },
    plaintext,
  };
}

// Fingerprint generation
function generateFingerprint(publicKey: Uint8Array): string {
  const hash = sodium.crypto_generichash(32, publicKey);
  return sodium.to_hex(hash);
}

// Safety number computation
function computeSafetyNumber(ourKey: Uint8Array, theirKey: Uint8Array): string {
  const combined = new Uint8Array(ourKey.length + theirKey.length);
  
  // Sort keys for consistent ordering
  if (sodium.compare(ourKey, theirKey) < 0) {
    combined.set(ourKey, 0);
    combined.set(theirKey, ourKey.length);
  } else {
    combined.set(theirKey, 0);
    combined.set(ourKey, theirKey.length);
  }
  
  const hash = sodium.crypto_generichash(32, combined);
  
  // Convert to display format (groups of digits)
  let result = '';
  for (let i = 0; i < hash.length; i += 4) {
    const chunk = (hash[i] << 24) | (hash[i+1] << 16) | (hash[i+2] << 8) | hash[i+3];
    result += (Math.abs(chunk) % 100000).toString().padStart(5, '0') + ' ';
  }
  
  return result.trim();
}

// Utility: Convert to/from base64
function toBase64(data: Uint8Array): string {
  return sodium.to_base64(data, sodium.base64_variants.URLSAFE_NO_PADDING);
}

function fromBase64(data: string): Uint8Array {
  return sodium.from_base64(data, sodium.base64_variants.URLSAFE_NO_PADDING);
}

// Export worker API
export {
  init,
  generateIdentityKeyPair,
  generateEphemeralKeyPair,
  generateSignedPreKey,
  x3dhInitiate,
  ratchetInit,
  ratchetEncrypt,
  ratchetDecrypt,
  generateFingerprint,
  computeSafetyNumber,
  hkdfDerive,
  toBase64,
  fromBase64,
};

export type {
  KeyPair,
  IdentityKeyPair,
  PreKeyBundle,
  RatchetState,
  EncryptedMessage,
};
