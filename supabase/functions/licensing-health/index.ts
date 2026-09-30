import { createClient } from "npm:@supabase/supabase-js@2.117.2";
import { jsonResponse, requiredEnvironment } from "../_shared/http.ts";

Deno.serve(async (request) => {
  if (request.method !== "GET") {
    return jsonResponse(405, { status: "error" });
  }
  try {
    const admin = createClient(
      requiredEnvironment("SUPABASE_URL"),
      requiredEnvironment("SUPABASE_SERVICE_ROLE_KEY"),
      { auth: { persistSession: false, autoRefreshToken: false } },
    );
    const { data, error } = await admin.rpc("licensing_health");
    if (error || data !== true) throw new Error("DATABASE_UNHEALTHY");
    return jsonResponse(200, { status: "ok", schemaVersion: 1 });
  } catch {
    return jsonResponse(503, { status: "unavailable" });
  }
});
