import { useState } from 'react';
import { Shield, Bell, Eye, Globe, Trash2, Copy, Check } from 'lucide-react';
import { useAuthStore } from '../stores/authStore';

export function SettingsPage() {
  const { identity, logout } = useAuthStore();
  const [copied, setCopied] = useState(false);
  const [settings, setSettings] = useState({
    sendReadReceipts: true,
    sendTypingIndicators: true,
    showOnlineStatus: true,
    notifications: true,
    useTor: false,
  });

  const copyFingerprint = async () => {
    if (identity?.fingerprint) {
      await navigator.clipboard.writeText(identity.fingerprint);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const toggleSetting = (key: keyof typeof settings) => {
    setSettings((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  return (
    <div className="max-w-2xl mx-auto p-6">
      <h1 className="text-2xl font-bold mb-6">Settings</h1>

      {/* Identity Section */}
      <section className="bg-gray-800 rounded-lg p-6 mb-6">
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <Shield className="w-5 h-5 text-emerald-400" />
          Identity
        </h2>

        <div className="space-y-4">
          <div>
            <label className="text-sm text-gray-400">Your Fingerprint</label>
            <div className="mt-1 flex items-center gap-2">
              <code className="flex-1 p-3 bg-gray-900 rounded font-mono text-sm text-emerald-400 break-all">
                {identity?.fingerprint}
              </code>
              <button
                onClick={copyFingerprint}
                className="p-3 bg-gray-700 rounded hover:bg-gray-600 transition-colors"
              >
                {copied ? (
                  <Check className="w-5 h-5 text-emerald-400" />
                ) : (
                  <Copy className="w-5 h-5" />
                )}
              </button>
            </div>
            <p className="text-xs text-gray-500 mt-2">
              Share this fingerprint with contacts to verify your identity
            </p>
          </div>

          <div>
            <label className="text-sm text-gray-400">Public Key</label>
            <div className="mt-1">
              <code className="block p-3 bg-gray-900 rounded font-mono text-xs text-gray-400 break-all">
                {identity?.publicKey}
              </code>
            </div>
          </div>
        </div>
      </section>

      {/* Privacy Section */}
      <section className="bg-gray-800 rounded-lg p-6 mb-6">
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <Eye className="w-5 h-5 text-emerald-400" />
          Privacy
        </h2>

        <div className="space-y-4">
          <ToggleSetting
            label="Send Read Receipts"
            description="Let others know when you've read their messages"
            enabled={settings.sendReadReceipts}
            onToggle={() => toggleSetting('sendReadReceipts')}
          />
          <ToggleSetting
            label="Send Typing Indicators"
            description="Show when you're typing a message"
            enabled={settings.sendTypingIndicators}
            onToggle={() => toggleSetting('sendTypingIndicators')}
          />
          <ToggleSetting
            label="Show Online Status"
            description="Let others see when you're online"
            enabled={settings.showOnlineStatus}
            onToggle={() => toggleSetting('showOnlineStatus')}
          />
        </div>
      </section>

      {/* Network Section */}
      <section className="bg-gray-800 rounded-lg p-6 mb-6">
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <Globe className="w-5 h-5 text-emerald-400" />
          Network
        </h2>

        <div className="space-y-4">
          <ToggleSetting
            label="Use Tor Network"
            description="Route all traffic through Tor for maximum anonymity (slower)"
            enabled={settings.useTor}
            onToggle={() => toggleSetting('useTor')}
          />
        </div>
      </section>

      {/* Notifications Section */}
      <section className="bg-gray-800 rounded-lg p-6 mb-6">
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <Bell className="w-5 h-5 text-emerald-400" />
          Notifications
        </h2>

        <div className="space-y-4">
          <ToggleSetting
            label="Enable Notifications"
            description="Receive notifications for new messages"
            enabled={settings.notifications}
            onToggle={() => toggleSetting('notifications')}
          />
        </div>
      </section>

      {/* Danger Zone */}
      <section className="bg-red-900/20 border border-red-900 rounded-lg p-6">
        <h2 className="text-lg font-semibold mb-4 flex items-center gap-2 text-red-400">
          <Trash2 className="w-5 h-5" />
          Danger Zone
        </h2>

        <p className="text-sm text-gray-400 mb-4">
          These actions are irreversible. Your identity and all messages will be permanently deleted.
        </p>

        <button
          onClick={logout}
          className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded transition-colors"
        >
          Delete Account & Data
        </button>
      </section>
    </div>
  );
}

function ToggleSetting({
  label,
  description,
  enabled,
  onToggle,
}: {
  label: string;
  description: string;
  enabled: boolean;
  onToggle: () => void;
}) {
  return (
    <div className="flex items-center justify-between">
      <div>
        <p className="font-medium">{label}</p>
        <p className="text-sm text-gray-400">{description}</p>
      </div>
      <button
        onClick={onToggle}
        className={`relative w-12 h-6 rounded-full transition-colors ${
          enabled ? 'bg-emerald-600' : 'bg-gray-600'
        }`}
      >
        <span
          className={`absolute top-1 w-4 h-4 bg-white rounded-full transition-transform ${
            enabled ? 'left-7' : 'left-1'
          }`}
        />
      </button>
    </div>
  );
}
