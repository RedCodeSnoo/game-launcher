// Mise en forme des durées et des dates (français).

/** 135 → « 2 h 15 », 45 → « 45 min », 0 → « 0 min ». */
export function formatMinutes(minutes: number): string {
  const m = Math.max(0, Math.round(minutes));
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  const rest = m % 60;
  return rest === 0 ? `${h} h` : `${h} h ${String(rest).padStart(2, "0")}`;
}

export function formatSeconds(seconds: number): string {
  if (seconds > 0 && seconds < 60) return `${Math.round(seconds)} s`;
  return formatMinutes(seconds / 60);
}

export function formatDate(iso: string | null | undefined): string {
  if (!iso) return "Jamais";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "Jamais";
  return d.toLocaleDateString("fr-FR", { day: "numeric", month: "short", year: "numeric" });
}

export function formatDateTime(iso: string | null | undefined): string {
  if (!iso) return "Jamais";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "Jamais";
  return d.toLocaleString("fr-FR", { day: "numeric", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit" });
}

/** « aujourd'hui », « hier », « il y a 5 jours », puis la date. */
export function formatRelative(iso: string | null | undefined, now: Date = new Date()): string {
  if (!iso) return "Jamais";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "Jamais";
  const startOfDay = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const days = Math.round((startOfDay(now) - startOfDay(d)) / 86_400_000);
  if (days <= 0) return "Aujourd'hui";
  if (days === 1) return "Hier";
  if (days < 30) return `Il y a ${days} jours`;
  return formatDate(iso);
}

export function percent(part: number, total: number): number {
  return total > 0 ? Math.round((part / total) * 100) : 0;
}

/** Couleur stable dérivée d'un texte (jaquettes de remplacement). */
export function hueOf(text: string): number {
  let h = 0;
  for (let i = 0; i < text.length; i++) h = (h * 31 + text.charCodeAt(i)) % 360;
  return h;
}
