import { createBrowserRouter, Link, Navigate, RouterProvider } from "react-router-dom";
import { AppShell } from "@/components/layout/AppShell";
import { LoginForm } from "@/components/auth/LoginForm";
import { HomePage } from "@/pages/HomePage";
import { SetupPage } from "@/pages/SetupPage";
import { AdminPage } from "@/pages/AdminPage";
import { PlatformAuth } from "@/components/settings/PlatformAuth";
import { Preferences } from "@/components/settings/Preferences";
import { MapiDebugPage } from "@/pages/MapiDebugPage";
import { LlmChatPage } from "@/pages/LlmChatPage";
import { TaskCreate } from "@/components/task/TaskCreate";

const router = createBrowserRouter([
  { path: "/login", element: <LoginForm /> },
  {
    path: "/",
    element: <AppShell />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "setup", element: <SetupPage /> },
      { path: "task/new", element: <TaskCreate /> },
      { path: "settings", element: <SettingsPage /> },
      { path: "settings/platforms", element: <PlatformAuth /> },
      { path: "settings/preferences", element: <Preferences /> },
      { path: "settings/mapi", element: <MapiDebugPage /> },
      { path: "settings/llm", element: <LlmChatPage /> },
      { path: "admin", element: <AdminPage /> },
    ],
  },
  { path: "*", element: <Navigate to="/" replace /> },
]);

function SettingsPage() {
  return (
    <div className="p-4 space-y-4">
      <h2 className="text-lg font-semibold" style={{ color: "var(--admiral-text-primary)" }}>设置</h2>
      <nav className="flex flex-col gap-2">
        <Link to="/settings/platforms" className="text-primary hover:underline">平台授权（B 站）</Link>
        <Link to="/settings/preferences" className="text-primary hover:underline">偏好设置</Link>
        <Link to="/settings/mapi" className="text-primary hover:underline">MAPI 调试</Link>
        <Link to="/settings/llm" className="text-primary hover:underline">LLM 对话</Link>
        <Link to="/admin" className="text-primary hover:underline">管理（仅管理员）</Link>
      </nav>
    </div>
  );
}

export function Router() {
  return <RouterProvider router={router} />;
}
