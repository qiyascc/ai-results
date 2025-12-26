import { useEffect, useState } from 'react';
import { Routes, Route, Navigate } from 'react-router-dom';
import { useAuthStore } from './stores/authStore';
import { Layout } from './components/Layout';
import { LoginPage } from './pages/LoginPage';
import { ChatPage } from './pages/ChatPage';
import { SettingsPage } from './pages/SettingsPage';
import { VerifyPage } from './pages/VerifyPage';
import { LoadingScreen } from './components/LoadingScreen';

export default function App() {
  const [isLoading, setIsLoading] = useState(true);
  const { isAuthenticated, initialize } = useAuthStore();

  useEffect(() => {
    const init = async () => {
      try {
        await initialize();
      } catch (error) {
        console.error('Failed to initialize:', error);
      } finally {
        setIsLoading(false);
      }
    };
    init();
  }, [initialize]);

  if (isLoading) {
    return <LoadingScreen />;
  }

  return (
    <Routes>
      <Route
        path="/login"
        element={isAuthenticated ? <Navigate to="/" /> : <LoginPage />}
      />
      <Route
        path="/"
        element={
          isAuthenticated ? (
            <Layout>
              <ChatPage />
            </Layout>
          ) : (
            <Navigate to="/login" />
          )
        }
      />
      <Route
        path="/chat/:userId"
        element={
          isAuthenticated ? (
            <Layout>
              <ChatPage />
            </Layout>
          ) : (
            <Navigate to="/login" />
          )
        }
      />
      <Route
        path="/settings"
        element={
          isAuthenticated ? (
            <Layout>
              <SettingsPage />
            </Layout>
          ) : (
            <Navigate to="/login" />
          )
        }
      />
      <Route
        path="/verify/:userId"
        element={
          isAuthenticated ? (
            <Layout>
              <VerifyPage />
            </Layout>
          ) : (
            <Navigate to="/login" />
          )
        }
      />
    </Routes>
  );
}
