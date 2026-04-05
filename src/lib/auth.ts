import { createSignal } from "solid-js";
import { createClient, type User } from "@supabase/supabase-js";
import { Store } from "@tauri-apps/plugin-store";

const SUPABASE_URL = "https://hsxotvzljoxeibdnoccr.supabase.co";
const SUPABASE_ANON_KEY = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6ImhzeG90dnpsam94ZWliZG5vY2NyIiwicm9sZSI6ImFub24iLCJpYXQiOjE3NzQ0MjU3MjQsImV4cCI6MjA5MDAwMTcyNH0.knlWrZ_h6HefW5SbHzMwiHjN94RC6P0lB-Rzwte__Uc";

export const supabase = createClient(SUPABASE_URL, SUPABASE_ANON_KEY);

export interface UserProfile {
  id: string;
  email: string;
  is_pro: boolean;
  subscription_status: string | null;
  pro_expires_at: string | null;
}

const [user, setUser] = createSignal<User | null>(null);
const [profile, setProfile] = createSignal<UserProfile | null>(null);
const [authLoading, setAuthLoading] = createSignal(true);

export { user, profile, authLoading };

// --- Profile cache (survives restart for offline grace period) ---

let _store: Store | null = null;

async function getStore(): Promise<Store> {
  if (!_store) _store = await Store.load("auth.json");
  return _store;
}

async function cacheProfile(p: UserProfile): Promise<void> {
  const store = await getStore();
  await store.set("profile", p);
  await store.save();
}

async function loadCachedProfile(): Promise<UserProfile | null> {
  try {
    const store = await getStore();
    return (await store.get<UserProfile>("profile")) ?? null;
  } catch {
    return null;
  }
}

async function clearCachedProfile(): Promise<void> {
  try {
    const store = await getStore();
    await store.delete("profile");
    await store.save();
  } catch {
    // Best-effort
  }
}

// --- Supabase queries ---

export async function fetchProfile(userId: string): Promise<UserProfile | null> {
  const { data, error } = await supabase
    .from("profiles")
    .select("id, email, is_pro, subscription_status, pro_expires_at")
    .eq("id", userId)
    .single();

  if (error || !data) return null;
  return data as UserProfile;
}

export function isPro(): boolean {
  const p = profile();
  if (!p) return false;
  if (p.is_pro) return true;

  // Offline grace period: allow Pro if pro_expires_at is in the future
  if (p.pro_expires_at) {
    return new Date(p.pro_expires_at) > new Date();
  }

  return false;
}

export async function signIn(
  email: string,
  password: string
): Promise<{ error: string | null }> {
  const { data, error } = await supabase.auth.signInWithPassword({
    email,
    password,
  });

  if (error) return { error: error.message };

  setUser(data.user);
  if (data.user) {
    const p = await fetchProfile(data.user.id);
    setProfile(p);
    if (p) await cacheProfile(p);
  }

  return { error: null };
}

export async function signOut(): Promise<void> {
  await supabase.auth.signOut({ scope: "local" });
  setUser(null);
  setProfile(null);
  await clearCachedProfile();
}

// --- Initialization (idempotent) ---

let _initialized = false;

export async function initAuth(): Promise<void> {
  if (_initialized) return;
  _initialized = true;

  setAuthLoading(true);

  // Load cached profile first (offline resilience)
  const cached = await loadCachedProfile();
  if (cached) setProfile(cached);

  const {
    data: { session },
  } = await supabase.auth.getSession();

  if (session?.user) {
    setUser(session.user);
    const p = await fetchProfile(session.user.id);
    if (p) {
      setProfile(p);
      await cacheProfile(p);
    }
    // If fetch failed, keep the cached profile (already set above)
  }

  // Synchronous callback -- kick off async profile load outside the lock
  const { data: { subscription } } = supabase.auth.onAuthStateChange((_event, session) => {
    if (session?.user) {
      setUser(session.user);
      void fetchProfile(session.user.id).then((p) => {
        if (p) {
          setProfile(p);
          void cacheProfile(p);
        }
      });
    } else {
      setUser(null);
      setProfile(null);
      void clearCachedProfile();
    }
  });

  // Store subscription for potential cleanup
  void subscription;

  setAuthLoading(false);
}
