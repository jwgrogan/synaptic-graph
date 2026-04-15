import { invoke } from "@tauri-apps/api/core";
import type {
  Impulse,
  Connection,
  MemoryStats,
  SearchResult,
  ImpulseDetail,
  GhostSource,
} from "./types";

export async function getAllImpulses(): Promise<Impulse[]> {
  return invoke<Impulse[]>("get_all_impulses");
}

export async function getAllConnections(): Promise<Connection[]> {
  return invoke<Connection[]>("get_all_connections");
}

export async function getMemoryStats(): Promise<MemoryStats> {
  return invoke<MemoryStats>("get_memory_stats");
}

export async function searchMemories(
  query: string,
  maxResults?: number
): Promise<SearchResult> {
  return invoke<SearchResult>("search_memories", {
    query,
    maxResults: maxResults ?? 20,
  });
}

export async function getImpulseDetail(id: string): Promise<ImpulseDetail> {
  return invoke<ImpulseDetail>("get_impulse_detail", { id });
}

export async function getGhostSources(): Promise<GhostSource[]> {
  return invoke<GhostSource[]>("get_ghost_sources");
}

export async function getGhostNodes(
  sourceName: string
): Promise<Record<string, unknown>[]> {
  return invoke<Record<string, unknown>[]>("get_ghost_nodes", { sourceName });
}

export async function getAllTags(): Promise<{ name: string; color: string }[]> {
  return invoke("get_all_tags");
}

export async function getImpulseTags(
  impulseId: string
): Promise<{ name: string; color: string }[]> {
  return invoke("get_impulse_tags", { impulseId });
}
