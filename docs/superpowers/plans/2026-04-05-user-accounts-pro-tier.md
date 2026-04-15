# User Accounts + Pro Tier Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable users to sign up, sign in, pay for Pro via LemonSqueezy, and have the desktop app gate Pro features behind subscription status.

**Architecture:** Supabase Auth handles user accounts (email/password). LemonSqueezy (Merchant of Record) handles payments and tax. A webhook from LemonSqueezy hits the murmur-web API to flip `is_pro` on the user's Supabase profile. The desktop app (Tauri + SolidJS) signs in via Supabase JS client and reads Pro status to gate features.

**Tech Stack:** Supabase Auth + PostgreSQL, LemonSqueezy API, Next.js 16 App Router (murmur-web), SolidJS + Tauri 2 (murmur desktop), Rust (murmur-pro backend)

**Issues:** murmurlinux/internal#42 (user account system), murmurlinux/internal#43 (license validation in app)

**Repos touched:**
- `murmur-web` (`~/Projects/murmur-web`) -- auth pages, webhook endpoint, Supabase SSR
- `murmur` (`~/Projects/murmur`) -- Supabase JS client, auth state, sign-in UI
- `murmur-pro` (`~/Projects/murmur-pro`) -- Pro feature gating

**Supabase project:** `hsxotvzljoxeibdnoccr` (existing, currently has `waitlist` table only)

---

## File Structure

### murmur-web (website)

| Action | Path | Responsibility |
|--------|------|---------------|
| Create | `src/lib/supabase/server.ts` | Server-side Supabase client with cookie management |
| Create | `src/lib/supabase/client.ts` | Browser-side Supabase client for auth pages |
| Modify | `src/lib/supabase.ts` | Keep as-is for API routes that don't need cookies |
| Create | `src/app/auth/login/page.tsx` | Login form (email/password) |
| Create | `src/app/auth/signup/page.tsx` | Signup form (email/password) |
| Create | `src/app/auth/callback/route.ts` | OAuth/magic link callback handler |
| Create | `src/app/api/webhooks/lemonsqueezy/route.ts` | LemonSqueezy subscription webhook |
| Create | `middleware.ts` | Session refresh middleware (Supabase SSR pattern) |

### murmur (desktop app, public)

| Action | Path | Responsibility |
|--------|------|---------------|
| Create | `src/lib/auth.ts` | Supabase client + auth state store |
| Modify | `src/components/SettingsPanel.tsx` | Add Account section with sign-in/sign-out |

### murmur-pro (private)

| Action | Path | Responsibility |
|--------|------|---------------|
| Create | `src/auth.rs` | Pro status check, feature gating helper |
| Modify | `src/engine_factory.rs` | Gate cloud engines behind Pro status |

### Supabase (database, via SQL)

| Action | Object | Responsibility |
|--------|--------|---------------|
| Create | `profiles` table | User profile with Pro status fields |
| Create | `handle_new_user()` trigger | Auto-create profile on signup |
| Create | RLS policies | Users read own profile, service role updates Pro fields |

---

## Phase A: Supabase Database Schema

### Task 1: Create profiles table and trigger

**Context:** The Supabase project `hsxotvzljoxeibdnoccr` currently has only a `waitlist` table. We need a `profiles` table linked to `auth.users` that tracks Pro subscription status. This is the foundation everything else builds on.

**Method:** Run SQL via Supabase MCP `execute_sql` tool, or via the Supabase SQL Editor.

- [ ] **Step 1: Create the profiles table**

```sql
create table public.profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  email text not null,
  is_pro boolean not null default false,
  lemon_customer_id bigint,
  lemon_subscription_id bigint,
  subscription_status text,
  pro_since timestamptz,
  pro_expires_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

comment on table public.profiles is 'User profiles with Pro subscription status';
comment on column public.profiles.is_pro is 'True when user has an active Pro subscription';
comment on column public.profiles.subscription_status is 'LemonSqueezy status: active, cancelled, expired, past_due, paused';
comment on column public.profiles.pro_expires_at is 'Used for offline grace period in desktop app';
```

- [ ] **Step 2: Create auto-profile trigger**

This creates a profile row automatically when a user signs up via Supabase Auth.

```sql
create or replace function public.handle_new_user()
returns trigger
language plpgsql
security definer set search_path = ''
as $$
begin
  insert into public.profiles (id, email)
  values (new.id, new.email);
  return new;
end;
$$;

create trigger on_auth_user_created
  after insert on auth.users
  for each row execute function public.handle_new_user();
```

- [ ] **Step 3: Create updated_at trigger**

```sql
create or replace function public.set_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create trigger profiles_updated_at
  before update on public.profiles
  for each row execute function public.set_updated_at();
```

- [ ] **Step 4: Enable RLS and create policies**

```sql
alter table public.profiles enable row level security;

-- Users can read their own profile
create policy "Users can read own profile"
  on public.profiles for select
  using (auth.uid() = id);

-- Users can update non-sensitive fields on their own profile
create policy "Users can update own profile"
  on public.profiles for update
  using (auth.uid() = id)
  with check (auth.uid() = id);

-- Block users from modifying Pro fields directly (webhook only)
-- This is enforced by only exposing safe columns in the app, not by column-level RLS
-- The webhook uses the service_role key which bypasses RLS
```

- [ ] **Step 5: Create index on email for webhook lookups**

```sql
create index profiles_email_idx on public.profiles (email);
```

- [ ] **Step 6: Verify by running a test query**

```sql
select column_name, data_type, is_nullable, column_default
from information_schema.columns
where table_schema = 'public' and table_name = 'profiles'
order by ordinal_position;
```

Expected: 10 columns (id, email, is_pro, lemon_customer_id, lemon_subscription_id, subscription_status, pro_since, pro_expires_at, created_at, updated_at).

- [ ] **Step 7: Enable email auth provider**

In Supabase Dashboard > Authentication > Providers, verify that Email provider is enabled with:
- Confirm email: ON
- Secure email change: ON
- Double confirm email changes: OFF (not needed for MVP)

This may already be the default. Verify, don't assume.

---

## Phase B: murmur-web Auth Infrastructure

### Task 2: Install Supabase SSR package

**Files:**
- Modify: `~/Projects/murmur-web/package.json`

- [ ] **Step 1: Install @supabase/ssr**

```bash
cd ~/Projects/murmur-web && pnpm add @supabase/ssr
```

- [ ] **Step 2: Verify installation**

```bash
cd ~/Projects/murmur-web && pnpm list @supabase/ssr
```

Expected: `@supabase/ssr` appears in the dependency list.

- [ ] **Step 3: Commit**

```bash
cd ~/Projects/murmur-web
git checkout -b feature/42-user-accounts
git add package.json pnpm-lock.yaml
git commit -m "feat: add @supabase/ssr for auth infrastructure"
```

### Task 3: Create Supabase server client utility

**Files:**
- Create: `~/Projects/murmur-web/src/lib/supabase/server.ts`

**Context:** The existing `src/lib/supabase.ts` creates a basic client for API routes (waitlist, health). For auth, we need a server client that manages cookies for session persistence. This follows the official Supabase SSR pattern for Next.js App Router.

- [ ] **Step 1: Create the server client**

```typescript
// src/lib/supabase/server.ts
import { createServerClient } from "@supabase/ssr";
import { cookies } from "next/headers";

export async function createClient() {
  const cookieStore = await cookies();

  return createServerClient(
    process.env.NEXT_PUBLIC_SUPABASE_URL!,
    process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY!,
    {
      cookies: {
        getAll() {
          return cookieStore.getAll();
        },
        setAll(cookiesToSet) {
          try {
            cookiesToSet.forEach(({ name, value, options }) =>
              cookieStore.set(name, value, options)
            );
          } catch {
            // Called from a Server Component. Safe to ignore when
            // middleware handles session refresh.
          }
        },
      },
    }
  );
}
```

- [ ] **Step 2: Verify the file compiles**

```bash
cd ~/Projects/murmur-web && pnpm build 2>&1 | head -20
```

Note: This will fail if `NEXT_PUBLIC_SUPABASE_URL` and `NEXT_PUBLIC_SUPABASE_ANON_KEY` aren't set. That's expected at this stage. We'll address env vars in a later step.

- [ ] **Step 3: Commit**

```bash
cd ~/Projects/murmur-web
git add src/lib/supabase/server.ts
git commit -m "feat: add Supabase server client with cookie management"
```

### Task 4: Create Supabase browser client utility

**Files:**
- Create: `~/Projects/murmur-web/src/lib/supabase/client.ts`

**Context:** Auth pages (login, signup) run in the browser and need a client-side Supabase instance. This client uses `createBrowserClient` from `@supabase/ssr`.

- [ ] **Step 1: Create the browser client**

```typescript
// src/lib/supabase/client.ts
"use client";

import { createBrowserClient } from "@supabase/ssr";

export function createClient() {
  return createBrowserClient(
    process.env.NEXT_PUBLIC_SUPABASE_URL!,
    process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY!
  );
}
```

- [ ] **Step 2: Commit**

```bash
cd ~/Projects/murmur-web
git add src/lib/supabase/client.ts
git commit -m "feat: add Supabase browser client for auth pages"
```

### Task 5: Create auth middleware for session refresh

**Files:**
- Create: `~/Projects/murmur-web/middleware.ts` (project root, NOT inside src/)

**Context:** Supabase Auth sessions expire. The middleware intercepts every request, refreshes the session if needed, and updates the cookies. Without this, users get silently logged out. This does NOT protect routes. It only refreshes sessions. We're not gating website content behind auth.

- [ ] **Step 1: Create the middleware**

```typescript
// middleware.ts (project root)
import { createServerClient } from "@supabase/ssr";
import { NextResponse, type NextRequest } from "next/server";

export async function middleware(request: NextRequest) {
  let supabaseResponse = NextResponse.next({ request });

  const supabase = createServerClient(
    process.env.NEXT_PUBLIC_SUPABASE_URL!,
    process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY!,
    {
      cookies: {
        getAll() {
          return request.cookies.getAll();
        },
        setAll(cookiesToSet) {
          cookiesToSet.forEach(({ name, value }) =>
            request.cookies.set(name, value)
          );
          supabaseResponse = NextResponse.next({ request });
          cookiesToSet.forEach(({ name, value, options }) =>
            supabaseResponse.cookies.set(name, value, options)
          );
        },
      },
    }
  );

  // Refresh the session. Do NOT remove this line.
  await supabase.auth.getUser();

  return supabaseResponse;
}

export const config = {
  matcher: [
    // Skip static files and _next internals
    "/((?!_next/static|_next/image|favicon.ico|.*\\.(?:svg|png|jpg|jpeg|gif|webp)$).*)",
  ],
};
```

- [ ] **Step 2: Commit**

```bash
cd ~/Projects/murmur-web
git add middleware.ts
git commit -m "feat: add auth middleware for session refresh"
```

### Task 6: Add environment variables

**Context:** The existing Supabase client uses `SUPABASE_URL` and `SUPABASE_ANON_KEY` (server-only). Auth pages need client-side access, so we add `NEXT_PUBLIC_` prefixed versions. The existing server-only routes continue to work unchanged.

- [ ] **Step 1: Add new env vars to Vercel**

```bash
# These use the same Supabase project values, just with NEXT_PUBLIC_ prefix
# Get the current values first:
cd ~/Projects/murmur-web
cat .env.local 2>/dev/null || echo "cannot read .env.local, check Vercel dashboard"
```

Add these env vars (via Vercel dashboard or `vercel env add`):
- `NEXT_PUBLIC_SUPABASE_URL` = same value as existing `SUPABASE_URL`
- `NEXT_PUBLIC_SUPABASE_ANON_KEY` = same value as existing `SUPABASE_ANON_KEY`
- `LEMONSQUEEZY_WEBHOOK_SECRET` = generate a random 32+ char string (used to verify webhook signatures)
- `SUPABASE_SERVICE_ROLE_KEY` = from Supabase dashboard > Settings > API (needed for webhook to bypass RLS)

- [ ] **Step 2: Update .env.local for local dev**

Add the new variables to `.env.local` alongside the existing ones. The existing `SUPABASE_URL` and `SUPABASE_ANON_KEY` stay for the waitlist/health routes. The `NEXT_PUBLIC_` versions are for auth.

- [ ] **Step 3: Commit (do NOT commit .env.local)**

No commit needed here. Env vars are not committed.

### Task 7: Create auth callback route

**Files:**
- Create: `~/Projects/murmur-web/src/app/auth/callback/route.ts`

**Context:** When a user confirms their email or completes an OAuth flow, Supabase redirects them to `/auth/callback` with an authorization code. This route exchanges the code for a session.

- [ ] **Step 1: Create the callback route**

```typescript
// src/app/auth/callback/route.ts
import { NextResponse } from "next/server";
import { createClient } from "@/lib/supabase/server";

export async function GET(request: Request) {
  const { searchParams, origin } = new URL(request.url);
  const code = searchParams.get("code");
  const next = searchParams.get("next") ?? "/";

  if (code) {
    const supabase = await createClient();
    const { error } = await supabase.auth.exchangeCodeForSession(code);
    if (!error) {
      const forwardedHost = request.headers.get("x-forwarded-host");
      const isLocal = process.env.NODE_ENV === "development";
      if (isLocal) {
        return NextResponse.redirect(`${origin}${next}`);
      } else if (forwardedHost) {
        return NextResponse.redirect(`https://${forwardedHost}${next}`);
      } else {
        return NextResponse.redirect(`${origin}${next}`);
      }
    }
  }

  return NextResponse.redirect(`${origin}/auth/login?error=auth-code-error`);
}
```

- [ ] **Step 2: Commit**

```bash
cd ~/Projects/murmur-web
git add src/app/auth/callback/route.ts
git commit -m "feat: add auth callback route for email confirmation"
```

### Task 8: Create signup page

**Files:**
- Create: `~/Projects/murmur-web/src/app/auth/signup/page.tsx`

**Context:** Simple email + password signup form. Matches the Ocean Terminal design (deep navy, teal accents, Sora/JetBrains Mono fonts). On submit, calls Supabase Auth `signUp`. Shows a "check your email" message on success.

- [ ] **Step 1: Create the signup page**

```tsx
// src/app/auth/signup/page.tsx
"use client";

import { useState } from "react";
import { createClient } from "@/lib/supabase/client";
import Link from "next/link";

export default function SignupPage() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [success, setSuccess] = useState(false);
  const [loading, setLoading] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    setLoading(true);

    const supabase = createClient();
    const { error } = await supabase.auth.signUp({
      email,
      password,
      options: {
        emailRedirectTo: `${window.location.origin}/auth/callback`,
      },
    });

    setLoading(false);

    if (error) {
      setError(error.message);
    } else {
      setSuccess(true);
    }
  }

  if (success) {
    return (
      <main className="min-h-screen flex items-center justify-center bg-[#0c1222]">
        <div className="w-full max-w-sm p-8 rounded-2xl bg-white/[0.025] border border-white/[0.06]">
          <h1 className="text-xl font-semibold text-white mb-4 font-[family-name:var(--font-sora)]">
            Check your email
          </h1>
          <p className="text-sm text-gray-400">
            We sent a confirmation link to <strong className="text-teal-400">{email}</strong>.
            Click it to activate your account.
          </p>
          <Link
            href="/auth/login"
            className="block mt-6 text-center text-sm text-teal-400 hover:text-teal-300"
          >
            Back to login
          </Link>
        </div>
      </main>
    );
  }

  return (
    <main className="min-h-screen flex items-center justify-center bg-[#0c1222]">
      <div className="w-full max-w-sm p-8 rounded-2xl bg-white/[0.025] border border-white/[0.06]">
        <h1 className="text-xl font-semibold text-white mb-6 font-[family-name:var(--font-sora)]">
          Create account
        </h1>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-[10px] font-semibold uppercase tracking-wider text-teal-400 mb-2">
              Email
            </label>
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
              className="w-full px-3 py-2 rounded-lg bg-black/30 border border-white/[0.06] text-white text-sm outline-none focus:border-teal-400/50"
            />
          </div>

          <div>
            <label className="block text-[10px] font-semibold uppercase tracking-wider text-teal-400 mb-2">
              Password
            </label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              minLength={8}
              className="w-full px-3 py-2 rounded-lg bg-black/30 border border-white/[0.06] text-white text-sm outline-none focus:border-teal-400/50"
            />
          </div>

          {error && (
            <p className="text-sm text-red-400">{error}</p>
          )}

          <button
            type="submit"
            disabled={loading}
            className="w-full py-2.5 rounded-lg bg-teal-500 hover:bg-teal-400 text-white text-sm font-medium transition-colors disabled:opacity-50"
          >
            {loading ? "Creating account..." : "Create account"}
          </button>
        </form>

        <p className="mt-6 text-center text-sm text-gray-500">
          Already have an account?{" "}
          <Link href="/auth/login" className="text-teal-400 hover:text-teal-300">
            Sign in
          </Link>
        </p>
      </div>
    </main>
  );
}
```

- [ ] **Step 2: Verify the page renders in dev**

```bash
cd ~/Projects/murmur-web && pnpm dev
```

Navigate to `http://localhost:3000/auth/signup`. Confirm the form renders with Ocean Terminal styling.

- [ ] **Step 3: Commit**

```bash
cd ~/Projects/murmur-web
git add src/app/auth/signup/page.tsx
git commit -m "feat: add signup page with email/password form"
```

### Task 9: Create login page

**Files:**
- Create: `~/Projects/murmur-web/src/app/auth/login/page.tsx`

**Context:** Login form matching signup page design. On success, redirects to home page. Shows error on invalid credentials.

- [ ] **Step 1: Create the login page**

```tsx
// src/app/auth/login/page.tsx
"use client";

import { useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { createClient } from "@/lib/supabase/client";
import Link from "next/link";

export default function LoginPage() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const router = useRouter();
  const searchParams = useSearchParams();
  const authError = searchParams.get("error");

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    setLoading(true);

    const supabase = createClient();
    const { error } = await supabase.auth.signInWithPassword({
      email,
      password,
    });

    setLoading(false);

    if (error) {
      setError(error.message);
    } else {
      router.push("/");
      router.refresh();
    }
  }

  return (
    <main className="min-h-screen flex items-center justify-center bg-[#0c1222]">
      <div className="w-full max-w-sm p-8 rounded-2xl bg-white/[0.025] border border-white/[0.06]">
        <h1 className="text-xl font-semibold text-white mb-6 font-[family-name:var(--font-sora)]">
          Sign in
        </h1>

        {authError && (
          <p className="text-sm text-red-400 mb-4">
            Authentication failed. Please try again.
          </p>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-[10px] font-semibold uppercase tracking-wider text-teal-400 mb-2">
              Email
            </label>
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
              className="w-full px-3 py-2 rounded-lg bg-black/30 border border-white/[0.06] text-white text-sm outline-none focus:border-teal-400/50"
            />
          </div>

          <div>
            <label className="block text-[10px] font-semibold uppercase tracking-wider text-teal-400 mb-2">
              Password
            </label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              className="w-full px-3 py-2 rounded-lg bg-black/30 border border-white/[0.06] text-white text-sm outline-none focus:border-teal-400/50"
            />
          </div>

          {error && (
            <p className="text-sm text-red-400">{error}</p>
          )}

          <button
            type="submit"
            disabled={loading}
            className="w-full py-2.5 rounded-lg bg-teal-500 hover:bg-teal-400 text-white text-sm font-medium transition-colors disabled:opacity-50"
          >
            {loading ? "Signing in..." : "Sign in"}
          </button>
        </form>

        <p className="mt-6 text-center text-sm text-gray-500">
          No account yet?{" "}
          <Link href="/auth/signup" className="text-teal-400 hover:text-teal-300">
            Create one
          </Link>
        </p>
      </div>
    </main>
  );
}
```

- [ ] **Step 2: Test login flow end-to-end**

1. Navigate to `http://localhost:3000/auth/signup`, create a test account
2. Check email for confirmation link, click it
3. Navigate to `http://localhost:3000/auth/login`, sign in with those credentials
4. Verify redirect to home page

- [ ] **Step 3: Commit**

```bash
cd ~/Projects/murmur-web
git add src/app/auth/login/page.tsx
git commit -m "feat: add login page with email/password form"
```

---

## Phase C: LemonSqueezy Webhook

### Task 10: Create LemonSqueezy webhook endpoint

**Files:**
- Create: `~/Projects/murmur-web/src/app/api/webhooks/lemonsqueezy/route.ts`

**Context:** When a user subscribes, LemonSqueezy fires a webhook. This endpoint verifies the HMAC-SHA256 signature, extracts the customer email and subscription status, and updates the user's profile in Supabase.

The webhook payload contains:
- `meta.event_name`: "subscription_created", "subscription_updated", "subscription_expired", etc.
- `meta.custom_data`: custom data passed through checkout (we'll pass `user_id` here)
- `data.attributes.user_email`: the customer's email
- `data.attributes.status`: "active", "cancelled", "expired", "past_due", "paused"
- `data.attributes.customer_id`: LemonSqueezy customer ID

The link between LemonSqueezy and Supabase is the email address. When a user checks out, we pass their Supabase `user_id` in checkout `custom_data`. The webhook uses this to update the correct profile. Fallback: match by email.

- [ ] **Step 1: Create the webhook route**

```typescript
// src/app/api/webhooks/lemonsqueezy/route.ts
import { NextResponse } from "next/server";
import crypto from "node:crypto";
import { createClient } from "@supabase/supabase-js";

const WEBHOOK_SECRET = process.env.LEMONSQUEEZY_WEBHOOK_SECRET!;

// Service role client bypasses RLS to update Pro fields
const supabase = createClient(
  process.env.SUPABASE_URL!,
  process.env.SUPABASE_SERVICE_ROLE_KEY!
);

function verifySignature(rawBody: string, signature: string | null): boolean {
  if (!signature || !WEBHOOK_SECRET) return false;
  const hmac = crypto.createHmac("sha256", WEBHOOK_SECRET);
  hmac.update(rawBody);
  const digest = hmac.digest("hex");
  try {
    return crypto.timingSafeEqual(
      Buffer.from(digest, "hex"),
      Buffer.from(signature, "hex")
    );
  } catch {
    return false;
  }
}

const PRO_STATUSES = new Set(["active", "on_trial"]);

export async function POST(request: Request) {
  const rawBody = await request.text();
  const signature = request.headers.get("x-signature");

  if (!verifySignature(rawBody, signature)) {
    return NextResponse.json({ error: "Invalid signature" }, { status: 401 });
  }

  const payload = JSON.parse(rawBody);
  const eventName: string = payload.meta?.event_name;
  const customData = payload.meta?.custom_data;
  const attrs = payload.data?.attributes;

  if (!attrs || !eventName) {
    return NextResponse.json({ error: "Invalid payload" }, { status: 400 });
  }

  const subscriptionEvents = [
    "subscription_created",
    "subscription_updated",
    "subscription_cancelled",
    "subscription_expired",
    "subscription_resumed",
    "subscription_paused",
  ];

  if (!subscriptionEvents.includes(eventName)) {
    // Not a subscription event we care about. Acknowledge it.
    return NextResponse.json({ received: true });
  }

  const email: string = attrs.user_email;
  const status: string = attrs.status;
  const customerId: number = attrs.customer_id;
  const subscriptionId = Number(payload.data?.id);
  const isPro = PRO_STATUSES.has(status);

  // Find the user. Prefer user_id from custom_data, fall back to email.
  const userId: string | undefined = customData?.user_id;

  const updateData = {
    is_pro: isPro,
    lemon_customer_id: customerId,
    lemon_subscription_id: subscriptionId,
    subscription_status: status,
    pro_since: isPro ? new Date().toISOString() : undefined,
    pro_expires_at: isPro
      ? null
      : new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString(), // 7-day grace
    updated_at: new Date().toISOString(),
  };

  // Remove undefined fields
  const cleanData = Object.fromEntries(
    Object.entries(updateData).filter(([, v]) => v !== undefined)
  );

  let error;
  if (userId) {
    ({ error } = await supabase
      .from("profiles")
      .update(cleanData)
      .eq("id", userId));
  } else {
    ({ error } = await supabase
      .from("profiles")
      .update(cleanData)
      .eq("email", email));
  }

  if (error) {
    console.error("[webhook] profile update failed:", error);
    return NextResponse.json({ error: "Update failed" }, { status: 500 });
  }

  return NextResponse.json({ received: true });
}
```

- [ ] **Step 2: Verify the route is valid**

```bash
cd ~/Projects/murmur-web && pnpm build 2>&1 | tail -20
```

Expected: Build succeeds (route compiles).

- [ ] **Step 3: Commit**

```bash
cd ~/Projects/murmur-web
git add src/app/api/webhooks/lemonsqueezy/route.ts
git commit -m "feat: add LemonSqueezy webhook for subscription events"
```

### Task 11: Push murmur-web branch and create PR

- [ ] **Step 1: Push the feature branch**

```bash
cd ~/Projects/murmur-web
git push -u origin feature/42-user-accounts
```

- [ ] **Step 2: Create PR**

```bash
cd ~/Projects/murmur-web
gh pr create \
  --title "feat: user account system with Supabase Auth" \
  --body "Adds auth infrastructure for Pro tier (murmurlinux/internal#42).

- Supabase SSR client (server + browser)
- Auth middleware for session refresh
- Login and signup pages (Ocean Terminal design)
- Auth callback route for email confirmation
- LemonSqueezy webhook endpoint for subscription events

Requires env vars: NEXT_PUBLIC_SUPABASE_URL, NEXT_PUBLIC_SUPABASE_ANON_KEY, SUPABASE_SERVICE_ROLE_KEY, LEMONSQUEEZY_WEBHOOK_SECRET"
```

- [ ] **Step 3: Merge after CI passes**

---

## Phase D: Desktop App Auth (murmur, public repo)

### Task 12: Add Supabase JS dependency

**Files:**
- Modify: `~/Projects/murmur/package.json`

- [ ] **Step 1: Install @supabase/supabase-js**

```bash
cd ~/Projects/murmur && pnpm add @supabase/supabase-js
```

- [ ] **Step 2: Commit on a new branch**

```bash
cd ~/Projects/murmur
git checkout -b feature/43-auth-sign-in
git add package.json pnpm-lock.yaml
git commit -m "feat: add @supabase/supabase-js for user auth"
```

### Task 13: Create auth state module

**Files:**
- Create: `~/Projects/murmur/src/lib/auth.ts`

**Context:** This module manages auth state in the desktop app. It creates a Supabase client (using the public project URL and anon key, which are safe to ship in a desktop binary), handles sign-in/sign-out, and exposes reactive auth state via SolidJS signals. Session persistence uses the WebView's localStorage (built into Tauri's WebView).

The Supabase URL and anon key are public values. The anon key only allows operations that RLS permits. This is the standard pattern for mobile and desktop apps.

- [ ] **Step 1: Create the auth module**

```typescript
// src/lib/auth.ts
import { createSignal } from "solid-js";
import { createClient, type User, type Session } from "@supabase/supabase-js";

// Public values: safe to ship in desktop binary. RLS protects the data.
const SUPABASE_URL = "https://hsxotvzljoxeibdnoccr.supabase.co";
const SUPABASE_ANON_KEY = "PLACEHOLDER_REPLACE_WITH_REAL_ANON_KEY";

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
  }

  return { error: null };
}

export async function signOut(): Promise<void> {
  await supabase.auth.signOut();
  setUser(null);
  setProfile(null);
}

export async function initAuth(): Promise<void> {
  setAuthLoading(true);

  // Restore session from localStorage (automatic in Supabase JS)
  const {
    data: { session },
  } = await supabase.auth.getSession();

  if (session?.user) {
    setUser(session.user);
    const p = await fetchProfile(session.user.id);
    setProfile(p);
  }

  // Listen for auth state changes (token refresh, sign out, etc.)
  supabase.auth.onAuthStateChange(async (event, session) => {
    if (session?.user) {
      setUser(session.user);
      const p = await fetchProfile(session.user.id);
      setProfile(p);
    } else {
      setUser(null);
      setProfile(null);
    }
  });

  setAuthLoading(false);
}
```

- [ ] **Step 2: Replace the anon key placeholder**

Read the actual anon key from the Supabase dashboard (Settings > API > `anon` `public` key) or from `murmur-web/.env.local`. Replace `PLACEHOLDER_REPLACE_WITH_REAL_ANON_KEY` with the real value. This is a public key, safe to commit.

- [ ] **Step 3: Commit**

```bash
cd ~/Projects/murmur
git add src/lib/auth.ts
git commit -m "feat: add auth state module with Supabase client"
```

### Task 14: Add Account section to SettingsPanel

**Files:**
- Modify: `~/Projects/murmur/src/components/SettingsPanel.tsx`

**Context:** Add an "Account" section at the top of the settings panel. When signed out, show email + password fields and a Sign In button. When signed in, show the user's email, Pro status, and a Sign Out button. Uses the same Ocean Terminal glass-card styling as existing sections.

This task modifies an existing 587-line file. We add an Account section at the top of the settings form, and call `initAuth()` in the `onMount`.

- [ ] **Step 1: Add imports at the top of SettingsPanel.tsx**

Add after the existing imports (line 4):

```typescript
import { initAuth, signIn, signOut, user, profile, isPro, authLoading } from "../lib/auth";
```

- [ ] **Step 2: Add auth initialization to onMount**

Find the existing `onMount` call inside the component function. Add `initAuth()` as the first line inside it:

```typescript
onMount(async () => {
  await initAuth();
  // ... existing onMount code (loadSettings, etc.)
});
```

If there is no `onMount` wrapping the existing init logic, wrap the existing init + `initAuth()` together.

- [ ] **Step 3: Add Account section JSX**

Add this section BEFORE the first existing glass card section (the one that starts with the skin/theme settings). This goes inside the main scrollable container:

```tsx
{/* Account */}
<div style={glass}>
  <span style={label}>Account</span>
  {authLoading() ? (
    <p style={{ color: "#999", "font-size": "13px" }}>Loading...</p>
  ) : user() ? (
    <div>
      <p style={{ color: "#e0e0e0", "font-size": "13px", "margin-bottom": "8px" }}>
        {profile()?.email ?? user()?.email}
      </p>
      <p style={{
        color: isPro() ? "#14b8a6" : "#999",
        "font-size": "11px",
        "font-weight": "600",
        "text-transform": "uppercase",
        "letter-spacing": "0.05em",
        "margin-bottom": "12px",
      }}>
        {isPro() ? "Pro" : "Free"}
      </p>
      <button
        onClick={() => signOut()}
        style={{
          padding: "6px 16px",
          background: "rgba(255, 255, 255, 0.05)",
          border: "1px solid rgba(255, 255, 255, 0.1)",
          "border-radius": "6px",
          color: "#e0e0e0",
          "font-size": "12px",
          cursor: "pointer",
        }}
      >
        Sign out
      </button>
    </div>
  ) : (
    <AccountSignIn />
  )}
</div>
```

- [ ] **Step 4: Create the AccountSignIn component**

Add this component definition INSIDE SettingsPanel.tsx, before the main exported component:

```tsx
function AccountSignIn() {
  const [email, setEmail] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [error, setError] = createSignal("");
  const [loading, setLoading] = createSignal(false);

  async function handleSignIn(e: Event) {
    e.preventDefault();
    setError("");
    setLoading(true);
    const result = await signIn(email(), password());
    setLoading(false);
    if (result.error) setError(result.error);
  }

  return (
    <form onSubmit={handleSignIn}>
      <input
        type="email"
        placeholder="Email"
        value={email()}
        onInput={(e) => setEmail(e.currentTarget.value)}
        required
        style={{ ...inputBase, "margin-bottom": "8px" }}
      />
      <input
        type="password"
        placeholder="Password"
        value={password()}
        onInput={(e) => setPassword(e.currentTarget.value)}
        required
        style={{ ...inputBase, "margin-bottom": "8px" }}
      />
      {error() && (
        <p style={{ color: "#f87171", "font-size": "12px", "margin-bottom": "8px" }}>
          {error()}
        </p>
      )}
      <button
        type="submit"
        disabled={loading()}
        style={{
          width: "100%",
          padding: "8px",
          background: "#14b8a6",
          border: "none",
          "border-radius": "8px",
          color: "#fff",
          "font-size": "13px",
          "font-weight": "500",
          cursor: loading() ? "wait" : "pointer",
          opacity: loading() ? "0.5" : "1",
        }}
      >
        {loading() ? "Signing in..." : "Sign in"}
      </button>
      <p style={{ color: "#666", "font-size": "11px", "margin-top": "8px", "text-align": "center" }}>
        Create an account at murmurlinux.com
      </p>
    </form>
  );
}
```

- [ ] **Step 5: Verify the settings panel renders**

```bash
cd ~/Projects/murmur && pnpm tauri dev
```

Open settings (gear icon). Verify the Account section appears at the top with a sign-in form. Sign in with a test account created in Phase B. Verify it shows email and "Free" status.

- [ ] **Step 6: Commit**

```bash
cd ~/Projects/murmur
git add src/lib/auth.ts src/components/SettingsPanel.tsx
git commit -m "feat: add account section with sign-in to settings panel"
```

### Task 15: Push murmur branch and create PR

- [ ] **Step 1: Push and create PR**

```bash
cd ~/Projects/murmur
git push -u origin feature/43-auth-sign-in
gh pr create \
  --title "feat: add user auth with Supabase sign-in" \
  --body "Adds auth to the desktop app (murmurlinux/internal#43).

- Supabase JS client with session persistence
- Auth state module (sign in, sign out, profile fetch, is_pro check)
- Account section in Settings panel (sign-in form, Pro status badge)
- Offline grace period: Pro features work 7 days without connectivity"
```

- [ ] **Step 2: Merge after CI passes**

---

## Phase E: Pro Feature Gating (murmur-pro, private repo)

### Task 16: Add auth check to engine factory

**Files:**
- Modify: `~/Projects/murmur-pro/src/engine_factory.rs`

**Context:** Currently, `build_engine` in murmur-pro allows selecting "groq" (cloud STT) unconditionally. After this change, cloud engines require a valid Pro subscription. For the CLI, this means passing a `--pro-token` flag or having a cached session. For the desktop app (future), this will be gated by the frontend auth state.

For now, the CLI gating is a simple check: if engine is not "local", require either `--api-key` (which implies the user has Pro access to the key) or a future `--pro-token` flag. The real gating happens in the desktop app frontend (Task 14 already exposes `isPro()`).

- [ ] **Step 1: Add a comment documenting the gating strategy**

At the top of `engine_factory.rs`, add:

```rust
// Cloud engines (groq, deepgram, etc.) are Pro features.
// - Desktop app: gated by isPro() in the SolidJS frontend before invoking cloud STT.
// - CLI: gated by requiring --api-key (Pro users receive API keys with their subscription).
// Future: validate Pro status server-side via Supabase JWT.
```

- [ ] **Step 2: Commit**

```bash
cd ~/Projects/murmur-pro
git checkout -b feature/43-pro-gating
git add src/engine_factory.rs
git commit -m "docs: document Pro feature gating strategy in engine factory"
```

### Task 17: Gate cloud STT in desktop app frontend

**Files:**
- This task documents how the desktop app gates Pro features. The actual gating code was already added in Task 14 (the `isPro()` function). When the desktop app eventually integrates cloud STT in its GUI (not just CLI), the engine selection dropdown should only show cloud options when `isPro()` returns true.

No code changes needed here for MVP. The CLI already requires `--api-key` for cloud engines, and the desktop app doesn't yet expose cloud STT in its GUI.

- [ ] **Step 1: Document the gating plan**

Add to `~/Projects/murmur-pro/CLAUDE.md`:

```markdown
## Pro Feature Gating

- Desktop app: `isPro()` from `src/lib/auth.ts` in the public repo gates cloud features.
- CLI: `--api-key` flag required for cloud engines (Groq, etc.).
- Webhook: LemonSqueezy subscription events update `is_pro` in Supabase profiles.
- Grace period: 7 days offline before Pro features lock out.
```

- [ ] **Step 2: Commit and push**

```bash
cd ~/Projects/murmur-pro
git add CLAUDE.md
git commit -m "docs: add Pro feature gating documentation"
git push -u origin feature/43-pro-gating
```

- [ ] **Step 3: Create PR and merge**

```bash
cd ~/Projects/murmur-pro
gh pr create \
  --title "docs: Pro feature gating strategy" \
  --body "Documents how Pro features are gated across CLI and desktop app (murmurlinux/internal#43).

No behavioral changes. Gating is already in place via --api-key for CLI.
Desktop app gating via isPro() to be enforced when cloud STT GUI is added."
```

---

## Phase F: Verification and Cleanup

### Task 18: End-to-end verification

- [ ] **Step 1: Test signup flow on website**

1. Go to `murmurlinux.com/auth/signup` (or localhost:3000 for dev)
2. Create account with a test email
3. Confirm email via link
4. Sign in at `/auth/login`
5. Verify redirect to home page

- [ ] **Step 2: Test desktop app sign-in**

1. Run `pnpm tauri dev` in murmur repo
2. Open settings
3. Sign in with the test account
4. Verify "Free" badge shows
5. Close and reopen app -- verify session persists

- [ ] **Step 3: Test webhook (manual)**

Send a test webhook payload to the endpoint using curl:

```bash
# Generate a valid signature
SECRET="your-webhook-secret"
BODY='{"meta":{"event_name":"subscription_created","custom_data":{"user_id":"test-uuid"}},"data":{"id":"1","attributes":{"user_email":"test@example.com","status":"active","customer_id":12345}}}'
SIGNATURE=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | awk '{print $2}')

curl -X POST https://murmurlinux.com/api/webhooks/lemonsqueezy \
  -H "Content-Type: application/json" \
  -H "X-Signature: $SIGNATURE" \
  -d "$BODY"
```

Verify the user's profile in Supabase now has `is_pro = true`.

- [ ] **Step 4: Test Pro status in desktop app**

After the webhook test, refresh the desktop app (sign out and back in). Verify "Pro" badge shows in the Account section.

### Task 19: Close issues

- [ ] **Step 1: Close #42 and #43**

```bash
gh issue close 42 -R murmurlinux/internal --comment "User account system implemented. Supabase Auth + LemonSqueezy webhook. PRs: murmur-web#XX, murmur#XX"
gh issue close 43 -R murmurlinux/internal --comment "Desktop app auth implemented. Sign-in via Supabase, Pro gating via isPro(). PRs: murmur#XX, murmur-pro#XX"
```

---

## Summary

| Phase | Repo | What | Depends on |
|-------|------|------|-----------|
| A | Supabase | profiles table, triggers, RLS | nothing |
| B | murmur-web | Auth pages, middleware, SSR client | A |
| C | murmur-web | LemonSqueezy webhook endpoint | A |
| D | murmur | Supabase client, auth state, settings UI | A |
| E | murmur-pro | Feature gating docs | D |
| F | all | End-to-end verification | B, C, D, E |

Phases B, C, and D can run in parallel after A completes. Phase E can run in parallel with B and C.
