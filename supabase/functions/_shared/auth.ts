import { createClient } from "npm:@supabase/supabase-js@2.117.2";
import { requiredEnvironment } from "./http.ts";

export function licensingAdmin() {
  return createClient(requiredEnvironment("SUPABASE_URL"), requiredEnvironment("SUPABASE_SERVICE_ROLE_KEY"), {
    auth: { persistSession: false, autoRefreshToken: false },
  });
}

export async function authenticatedUserId(request: Request, admin: ReturnType<typeof licensingAdmin>): Promise<string | null> {
  const authorization = request.headers.get("authorization");
  if (!authorization?.startsWith("Bearer ")) return null;
  const { data, error } = await admin.auth.getUser(authorization.slice(7));
  return error ? null : data.user?.id ?? null;
}
