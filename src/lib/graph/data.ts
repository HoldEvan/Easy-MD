import type { NoteIndex } from "../../types";
import { resolveTarget } from "../markdown/links";

export interface AutoNode {
  id: string;
  title: string;
  degree: number;
  unresolved: boolean;
  x?: number;
  y?: number;
}
export interface AutoLink { source: string; target: string }
export interface AutoGraph { nodes: AutoNode[]; links: AutoLink[] }

/** Graph edges are undirected: repeated references and backlinks share one edge. */
export function buildAutoGraph(notes: NoteIndex[], showUnresolved: boolean): AutoGraph {
  const nodes = new Map<string, AutoNode>(notes.map(n => [n.path, {
    id: n.path, title: n.title, degree: 0, unresolved: false,
  }]));
  const links: AutoLink[] = [];
  const seen = new Set<string>();
  for (const n of notes) {
    for (const link of n.links) {
      const target = link.target.trim();
      if (!target || /^(?:[a-z][\w+.-]*:|\/\/)/i.test(target)) continue;
      const result = resolveTarget(target, notes, n.path);
      if (result.ambiguous) continue;
      let id = result.path;
      if (!id) {
        // Explicit non-Markdown extensions are resources, never missing notes.
        if (!showUnresolved || /\.(png|jpg|jpeg|gif|webp|svg|bmp|ico|avif|heic|tif|tiff|pdf|canvas|mp3|wav|ogg|m4a|flac|mp4|webm|mov|avi|mkv|zip|rar|7z|tar|gz|doc|docx|xls|xlsx|ppt|pptx)$/i.test(target)) continue;
        const normalized = target.replace(/\\/g, "/").replace(/\.(md|markdown)$/i, "");
        const scope = normalized.includes("/") ? n.path.slice(0, n.path.lastIndexOf("/") + 1) : "";
        id = `?${scope}${normalized.toLowerCase()}`;
        if (!nodes.has(id)) nodes.set(id, { id, title: normalized, degree: 0, unresolved: true });
      }
      if (!nodes.has(id) || id === n.path) continue;
      const key = JSON.stringify([n.path, id].sort());
      if (seen.has(key)) continue;
      seen.add(key);
      links.push({ source: n.path, target: id });
      nodes.get(n.path)!.degree++;
      nodes.get(id)!.degree++;
    }
  }
  return { nodes: [...nodes.values()], links };
}

/** Both incoming and outgoing references count toward a local neighborhood. */
export function localGraph(data: AutoGraph, root: string, depth: number): AutoGraph {
  if (!data.nodes.some(n => n.id === root)) return { nodes: [], links: [] };
  const included = new Set([root]);
  const adjacency = new Map<string, string[]>();
  for (const { source, target } of data.links) {
    adjacency.set(source, [...(adjacency.get(source) ?? []), target]);
    adjacency.set(target, [...(adjacency.get(target) ?? []), source]);
  }
  let frontier = [root];
  for (let i = 0; i < Math.min(3, Math.max(1, depth)); i++) {
    const next: string[] = [];
    for (const id of frontier) for (const neighbor of adjacency.get(id) ?? []) {
      if (!included.has(neighbor)) { included.add(neighbor); next.push(neighbor); }
    }
    frontier = next;
  }
  return {
    nodes: data.nodes.filter(n => included.has(n.id)),
    links: data.links.filter(l => included.has(l.source) && included.has(l.target)),
  };
}
