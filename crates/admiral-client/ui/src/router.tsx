import { createBrowserRouter, Navigate, RouterProvider } from "react-router-dom";
import { AppShell } from "@/components/layout/AppShell";
import { LoginForm } from "@/components/auth/LoginForm";
import { HomePage } from "@/pages/HomePage";
import { PlatformAuth } from "@/components/settings/PlatformAuth";
import { MapiDebugPage } from "@/pages/MapiDebugPage";
import { LlmChatPage } from "@/pages/LlmChatPage";

const router = createBrowserRouter([
  { path: "/login", element: <LoginForm /> },
  {
    path: "/",
    element: <AppShell />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "settings/platforms", element: <PlatformAuth /> },
      { path: "settings/mapi", element: <MapiDebugPage /> },
      { path: "settings/llm", element: <LlmChatPage /> },
    ],
  },
  { path: "*", element: <Navigate to="/" replace /> },
]);

export function Router() {
  return <RouterProvider router={router} />;
}
