import { ReactNode } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { MessageSquare, Settings, Shield, LogOut } from 'lucide-react';
import { useAuthStore } from '../stores/authStore';
import { cn } from '../lib/utils';

interface LayoutProps {
  children: ReactNode;
}

export function Layout({ children }: LayoutProps) {
  const location = useLocation();
  const { identity, logout } = useAuthStore();
  
  const navigation = [
    { name: 'Messages', href: '/', icon: MessageSquare },
    { name: 'Settings', href: '/settings', icon: Settings },
  ];
  
  return (
    <div className="flex h-screen bg-gray-900 text-white">
      {/* Sidebar */}
      <div className="w-64 bg-gray-800 border-r border-gray-700 flex flex-col">
        {/* Logo */}
        <div className="p-4 border-b border-gray-700">
          <h1 className="text-xl font-bold text-emerald-400 flex items-center gap-2">
            <Shield className="w-6 h-6" />
            QiyasHash
          </h1>
          <p className="text-xs text-gray-500 mt-1">E2E Encrypted</p>
        </div>
        
        {/* Navigation */}
        <nav className="flex-1 p-4 space-y-2">
          {navigation.map((item) => (
            <Link
              key={item.name}
              to={item.href}
              className={cn(
                'flex items-center gap-3 px-3 py-2 rounded-lg transition-colors',
                location.pathname === item.href
                  ? 'bg-emerald-600 text-white'
                  : 'text-gray-400 hover:bg-gray-700 hover:text-white'
              )}
            >
              <item.icon className="w-5 h-5" />
              {item.name}
            </Link>
          ))}
        </nav>
        
        {/* User info */}
        <div className="p-4 border-t border-gray-700">
          <div className="mb-3">
            <p className="text-sm text-gray-400">Your Fingerprint</p>
            <p className="font-mono text-xs text-emerald-400 truncate">
              {identity?.fingerprint.slice(0, 16)}...
            </p>
          </div>
          <button
            onClick={logout}
            className="flex items-center gap-2 text-sm text-gray-400 hover:text-red-400 transition-colors"
          >
            <LogOut className="w-4 h-4" />
            Sign Out
          </button>
        </div>
      </div>
      
      {/* Main content */}
      <main className="flex-1 overflow-hidden">{children}</main>
    </div>
  );
}
