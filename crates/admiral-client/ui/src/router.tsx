import { createBrowserRouter, Link, Navigate, RouterProvider } from "react-router-dom";
import { AppShell } from "@/components/layout/AppShell";
import { LoginForm } from "@/components/auth/LoginForm";
import { HomePage } from "@/pages/HomePage";
import { SetupPage } from "@/pages/SetupPage";
import { AdminPage } from "@/pages/AdminPage";
import { PlatformAuth } from "@/components/settings/PlatformAuth";
import { Preferences } from "@/components/settings/Preferences";
import { MapiDebugPage } from "@/pages/MapiDebugPage";
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
      { path: "admin", element: <AdminPage /> },
    ],
  },
  { path: "*", element: <Navigate to="/" replace /> },
]);

const SETTING_CARDS = [
  { to: "/settings/platforms", title: "平台授权", desc: "管理 B 站授权与账户绑定", tag: "OAuth" },
  { to: "/settings/preferences", title: "偏好", desc: "主题、语言与体验配置", tag: "Profile" },
  { to: "/settings/mapi", title: "MAPI", desc: "代理接口调试工具", tag: "Debug" },
  { to: "/admin", title: "管理", desc: "管理员功能入口", tag: "Admin" },
];

function SettingsPage() {
  return (
    <div className="admiral-grid-page">
      <div className="settings-panel" style={{ maxWidth: 980 }}>
      <h2 className="settings-title">设置中心</h2>
      <p className="settings-subtitle">统一管理授权、偏好与联调工具。</p>
      <div
        style={{
          marginTop: 14,
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))",
          gap: 12,
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
              gap: 8,
              borderRadius: 12,
              border: "1px solid var(--admiral-border)",
              background: "linear-gradient(180deg, #ffffff 0%, #f8fbff 100%)",
              textDecoration: "none",
              color: "var(--admiral-text-primary)",
            }}
          >
            <span className="admiral-chip admiral-chip-primary" style={{ width: "fit-content" }}>{c.tag}</span>
            <span style={{ fontSize: 15, fontWeight: 700 }}>{c.title}</span>
            <span style={{ fontSize: 13, color: "var(--admiral-text-secondary)" }}>{c.desc}</span>
          </Link>
        ))}
      </div>
      </div>
    </div>
  );
}

export function Router() {
  return <RouterProvider router={router} />;
}
