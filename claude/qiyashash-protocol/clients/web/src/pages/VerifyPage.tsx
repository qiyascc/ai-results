import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Shield, Check, X, QrCode } from 'lucide-react';
import { useAuthStore } from '../stores/authStore';
import * as crypto from '../crypto/worker';

export function VerifyPage() {
  const { userId } = useParams();
  const navigate = useNavigate();
  const { identity } = useAuthStore();
  const [safetyNumber, setSafetyNumber] = useState<string>('');
  const [isVerified, setIsVerified] = useState(false);
  const [theirFingerprint, setTheirFingerprint] = useState('');

  useEffect(() => {
    const computeSafetyNumber = async () => {
      if (identity && userId) {
        await crypto.init();
        
        // In production, fetch their public key from storage
        // For demo, use placeholder
        const ourKey = crypto.fromBase64(identity.publicKey);
        const theirKey = new Uint8Array(32).fill(0x42); // Placeholder
        
        const number = crypto.computeSafetyNumber(ourKey, theirKey);
        setSafetyNumber(number);
      }
    };
    
    computeSafetyNumber();
  }, [identity, userId]);

  const handleVerify = () => {
    setIsVerified(true);
    // In production, store verification status
    setTimeout(() => {
      navigate(`/chat/${userId}`);
    }, 1500);
  };

  const handleReject = () => {
    navigate('/');
  };

  // Format safety number for display
  const formatSafetyNumber = (number: string) => {
    const groups = number.split(' ');
    const rows = [];
    for (let i = 0; i < groups.length; i += 4) {
      rows.push(groups.slice(i, i + 4).join(' '));
    }
    return rows;
  };

  return (
    <div className="max-w-lg mx-auto p-6">
      <div className="text-center mb-8">
        <div className="inline-flex items-center justify-center w-16 h-16 bg-emerald-600 rounded-full mb-4">
          <Shield className="w-8 h-8 text-white" />
        </div>
        <h1 className="text-2xl font-bold">Verify Safety Number</h1>
        <p className="text-gray-400 mt-2">
          Verify that your messages are end-to-end encrypted
        </p>
      </div>

      <div className="bg-gray-800 rounded-lg p-6 mb-6">
        <p className="text-sm text-gray-400 mb-4">
          Compare these numbers with your contact in person or via a trusted channel.
          If they match, your conversation is secure.
        </p>

        {/* Safety Number Display */}
        <div className="bg-gray-900 rounded-lg p-4 mb-6">
          <div className="grid grid-cols-1 gap-2 font-mono text-lg text-center">
            {formatSafetyNumber(safetyNumber).map((row, i) => (
              <div key={i} className="text-emerald-400">{row}</div>
            ))}
          </div>
        </div>

        {/* QR Code Placeholder */}
        <div className="flex justify-center mb-6">
          <div className="w-48 h-48 bg-gray-900 rounded-lg flex items-center justify-center">
            <QrCode className="w-24 h-24 text-gray-600" />
          </div>
        </div>

        {/* Fingerprints */}
        <div className="space-y-4 text-sm">
          <div>
            <p className="text-gray-400 mb-1">Your fingerprint:</p>
            <code className="block p-2 bg-gray-900 rounded text-xs text-emerald-400 break-all">
              {identity?.fingerprint}
            </code>
          </div>
          <div>
            <p className="text-gray-400 mb-1">Their fingerprint:</p>
            <input
              type="text"
              value={theirFingerprint}
              onChange={(e) => setTheirFingerprint(e.target.value)}
              placeholder="Paste their fingerprint here to compare"
              className="w-full p-2 bg-gray-900 rounded text-xs text-white placeholder-gray-600 border border-gray-700 focus:border-emerald-500 focus:outline-none"
            />
          </div>
        </div>
      </div>

      {/* Action Buttons */}
      {isVerified ? (
        <div className="bg-emerald-600/20 border border-emerald-600 rounded-lg p-4 text-center">
          <Check className="w-8 h-8 text-emerald-400 mx-auto mb-2" />
          <p className="text-emerald-400 font-medium">Identity Verified!</p>
        </div>
      ) : (
        <div className="flex gap-4">
          <button
            onClick={handleReject}
            className="flex-1 py-3 px-4 bg-red-600/20 border border-red-600 text-red-400 rounded-lg hover:bg-red-600/30 transition-colors flex items-center justify-center gap-2"
          >
            <X className="w-5 h-5" />
            Numbers Don't Match
          </button>
          <button
            onClick={handleVerify}
            className="flex-1 py-3 px-4 bg-emerald-600 text-white rounded-lg hover:bg-emerald-700 transition-colors flex items-center justify-center gap-2"
          >
            <Check className="w-5 h-5" />
            Mark as Verified
          </button>
        </div>
      )}

      <p className="text-xs text-gray-500 text-center mt-6">
        Verification ensures that your messages can only be read by you and your contact.
        If the numbers don't match, your communication may be compromised.
      </p>
    </div>
  );
}
