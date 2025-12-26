import { useState } from 'react';
import { Shield, Key, Loader2 } from 'lucide-react';
import { useAuthStore } from '../stores/authStore';

export function LoginPage() {
  const [isCreating, setIsCreating] = useState(false);
  const { createIdentity } = useAuthStore();

  const handleCreate = async () => {
    setIsCreating(true);
    try {
      await createIdentity();
    } catch (error) {
      console.error('Failed to create identity:', error);
    } finally {
      setIsCreating(false);
    }
  };

  return (
    <div className="min-h-screen bg-gray-900 flex items-center justify-center p-4">
      <div className="max-w-md w-full">
        {/* Logo */}
        <div className="text-center mb-8">
          <div className="inline-flex items-center justify-center w-20 h-20 bg-emerald-600 rounded-full mb-4">
            <Shield className="w-10 h-10 text-white" />
          </div>
          <h1 className="text-3xl font-bold text-white">QiyasHash</h1>
          <p className="text-gray-400 mt-2">End-to-End Encrypted Messaging</p>
        </div>

        {/* Card */}
        <div className="bg-gray-800 rounded-2xl p-8 border border-gray-700">
          <h2 className="text-xl font-semibold text-white mb-6 text-center">
            Welcome to QiyasHash
          </h2>

          {/* Features */}
          <div className="space-y-4 mb-8">
            <Feature
              icon="🔐"
              title="End-to-End Encryption"
              description="Messages are encrypted on your device"
            />
            <Feature
              icon="🔑"
              title="Forward Secrecy"
              description="Past messages stay secure even if keys are compromised"
            />
            <Feature
              icon="👤"
              title="Deniability"
              description="Plausible deniability for all messages"
            />
            <Feature
              icon="🌐"
              title="Decentralized"
              description="No central server stores your messages"
            />
          </div>

          {/* Create button */}
          <button
            onClick={handleCreate}
            disabled={isCreating}
            className="w-full py-3 px-4 bg-emerald-600 hover:bg-emerald-700 disabled:bg-emerald-800 text-white font-medium rounded-lg flex items-center justify-center gap-2 transition-colors"
          >
            {isCreating ? (
              <>
                <Loader2 className="w-5 h-5 animate-spin" />
                Generating Keys...
              </>
            ) : (
              <>
                <Key className="w-5 h-5" />
                Create New Identity
              </>
            )}
          </button>

          <p className="text-xs text-gray-500 text-center mt-4">
            Your identity keys are generated locally and never leave your device
          </p>
        </div>

        {/* Footer */}
        <div className="text-center mt-8 text-gray-500 text-sm">
          <p>
            Open source •{' '}
            <a
              href="https://github.com/qiyascc/qiyashashchat"
              target="_blank"
              rel="noopener noreferrer"
              className="text-emerald-400 hover:underline"
            >
              GitHub
            </a>
          </p>
        </div>
      </div>
    </div>
  );
}

function Feature({
  icon,
  title,
  description,
}: {
  icon: string;
  title: string;
  description: string;
}) {
  return (
    <div className="flex items-start gap-3">
      <span className="text-2xl">{icon}</span>
      <div>
        <h3 className="font-medium text-white">{title}</h3>
        <p className="text-sm text-gray-400">{description}</p>
      </div>
    </div>
  );
}
