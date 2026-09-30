import { getVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";
import { version } from "../package.json";

export async function readAppVersion(): Promise<string> {
  // The installed executable is authoritative. Browser development uses build metadata.
  return isTauri() ? getVersion() : version;
}
