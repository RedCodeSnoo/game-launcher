// Assistant de premier lancement (§8.1) : launchers détectés → compte Steam → dossiers surveillés → premier scan.
import { useState } from "react";
import { Gamepad2 } from "lucide-react";
import { api } from "../api";
import { SteamAccountForm } from "./AccountsSection";
import { WatchedFoldersPanel } from "./AddGameDialog";
import { SourcesList } from "./Settings";
import { Button } from "./ui";

const STEPS = ["Launchers", "Compte Steam", "Jeux sans launcher", "Terminé"];

export function Wizard({ onDone }: { onDone: () => void }) {
  const [step, setStep] = useState(0);

  const finish = async () => {
    await api.setSetting("wizard_done", "1").catch(() => {});
    void api.refreshLibrary();
    onDone();
  };

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-bg/95 p-6" role="dialog" aria-modal="true" aria-label="Assistant de premier lancement">
      <div className="flex max-h-full w-full max-w-2xl flex-col rounded-2xl border border-line bg-panel shadow-2xl">
        <div className="flex items-center gap-3 border-b border-line px-6 py-4">
          <Gamepad2 size={26} className="text-accent" />
          <div className="flex-1">
            <h1 className="text-lg font-semibold">Bienvenue dans GameLauncher</h1>
            <p className="text-xs text-muted">
              Étape {step + 1} sur {STEPS.length} · {STEPS[step]}
            </p>
          </div>
          <button onClick={finish} className="text-xs text-muted hover:text-fg">
            Passer l'assistant
          </button>
        </div>
        <div className="flex gap-1 px-6 pt-4" aria-hidden>
          {STEPS.map((s, i) => (
            <span key={s} className={`h-1 flex-1 rounded-full ${i <= step ? "bg-accent" : "bg-panel2"}`} />
          ))}
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
          {step === 0 && (
            <>
              <p className="mb-3 text-sm text-muted">Voici les launchers trouvés sur ce PC. Leurs jeux installés sont ajoutés automatiquement, sur tous les disques. Vous pouvez désactiver une source.</p>
              <SourcesList compact />
            </>
          )}
          {step === 1 && (
            <>
              <p className="mb-3 text-sm text-muted">
                Facultatif. Avec une clé API Steam, les jeux possédés non installés, vos heures de jeu et vos succès apparaissent aussi. Votre profil Steam et ses « Détails des jeux » doivent être publics.
              </p>
              <SteamAccountForm />
            </>
          )}
          {step === 2 && (
            <>
              <p className="mb-3 text-sm text-muted">Facultatif. Indiquez les dossiers où se trouvent vos jeux sans launcher, puis validez les jeux proposés.</p>
              <WatchedFoldersPanel />
            </>
          )}
          {step === 3 && (
            <div className="flex flex-col items-center gap-2 py-8 text-center">
              <p className="text-base font-medium">Tout est prêt</p>
              <p className="max-w-md text-sm text-muted">
                La détection se relance à chaque démarrage et avec le bouton « Actualiser ». Vous pouvez ajouter un jeu à tout moment avec « Ajouter », ou en glissant un fichier .exe sur la fenêtre.
              </p>
            </div>
          )}
        </div>

        <div className="flex justify-between border-t border-line px-6 py-4">
          <Button onClick={() => setStep((s) => s - 1)} disabled={step === 0}>
            Précédent
          </Button>
          {step < STEPS.length - 1 ? (
            <Button variant="primary" onClick={() => setStep((s) => s + 1)}>
              {step === 0 ? "Continuer" : "Suivant"}
            </Button>
          ) : (
            <Button variant="primary" onClick={finish}>
              Ouvrir ma bibliothèque
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
