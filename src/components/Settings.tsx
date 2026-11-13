// Page Paramètres : une carte par rubrique.
import type { ReactNode } from "react";
import { SgdbForm, SteamAccountForm } from "./AccountsSection";

export function SettingsCard({ title, description, children }: { title: string; description?: string; children: ReactNode }) {
  return (
    <section className="rounded-xl border border-line bg-panel p-5">
      <h2 className="text-base font-semibold">{title}</h2>
      {description && <p className="mt-0.5 text-sm text-muted">{description}</p>}
      <div className="mt-4">{children}</div>
    </section>
  );
}

export function Settings({ children }: { children?: ReactNode }) {
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto flex max-w-3xl flex-col gap-5 px-6 py-5">
        <h1 className="text-lg font-semibold">Paramètres</h1>
        <SettingsCard title="Compte Steam" description="Affiche les jeux possédés non installés, les heures de jeu et les succès.">
          <SteamAccountForm />
        </SettingsCard>
        <SettingsCard title="Jaquettes – SteamGridDB" description="Les jeux Steam ont leurs visuels sans clé. SteamGridDB complète les autres sources.">
          <SgdbForm />
        </SettingsCard>
        {children}
      </div>
    </div>
  );
}
