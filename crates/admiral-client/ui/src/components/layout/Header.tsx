import { useAuthStore } from "@/stores/authStore";
import { useAuth } from "@/hooks/useAuth";

export function Header() {
  const user = useAuthStore((s) => s.user);
  const { logout } = useAuth();

  return (
    <header className="h-12 border-b flex items-center justify-between px-4 bg-white">
      <span className="font-medium">Admiral</span>
      <div className="flex items-center gap-2">
        {user && (
          <>
            <span className="text-sm text-gray-600">{user.email}</span>
            <button
              type="button"
              onClick={() => logout()}
              className="text-sm text-blue-600 hover:underline"
            >
              退出
            </button>
          </>
        )}
      </div>
    </header>
  );
}
