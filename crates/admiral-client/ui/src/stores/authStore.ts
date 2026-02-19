import { create } from "zustand";
import type { UserInfo, LicenseResponse } from "@/lib/types";

interface AuthState {
  user: UserInfo | null;
  license: LicenseResponse | null;
  setUser: (u: UserInfo | null) => void;
  setLicense: (l: LicenseResponse | null) => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  license: null,
  setUser: (user) => set({ user }),
  setLicense: (license) => set({ license }),
}));
