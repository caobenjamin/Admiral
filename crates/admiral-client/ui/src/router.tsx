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

const SETTING_CARDS = [
  { to: "/settings/platforms", icon: "📺", title: "平台授权", desc: "授权管理" },
  { to: "/settings/preferences", icon: "🎨", title: "偏好", desc: "主题语言" },
  { to: "/settings/mapi", icon: "🔌", title: "MAPI", desc: "调试工具" },
  { to: "/settings/llm", icon: "🤖", title: "LLM", desc: "调试工具" },
  { to: "/admin", icon: "⚙", title: "管理", desc: "仅管理员" },
];

function SettingsPage() {
  return (
    <div style={{ padding: 24, maxWidth: 720 }}>
      <h2 style={{ fontSize: 18, fontWeight: 600, color: "var(--admiral-text-primary)", marginBottom: 24 }}>
        ⚙ 设置
      </h2>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(160px, 1fr))",
          gap: 16,
        }}
      >
        {SETTING_CARDS.map((c) => (
          <Link
            key={c.to}
            to={c.to}
            className="settings-card"
            style={{
              display: "flex",
              flexDirection: "column",
              padding: 16,
              borderRadius: "var(--admiral-radius-card)",
              border: "1px solid var(--admiral-border)",
              backgroundColor: "var(--admiral-card)",
              boxShadow: "var(--admiral-shadow-card)",
              textDecoration: "none",
              color: "var(--admiral-text-primary)",
              transition: "transform 0.2s ease, box-shadow 0.2s ease",
            }}
          >
            <span style={{ fontSize: 24, marginBottom: 8 }}>{c.icon}</span>
            <span style={{ fontSize: 14, fontWeight: 600 }}>{c.title}</span>
            <span style={{ fontSize: 12, color: "var(--admiral-text-secondary)", marginTop: 4 }}>{c.desc}</span>
          </Link>
        ))}
      </div>
    </div>
  );
}

export function Router() {
  return <RouterProvider router={router} />;
}
