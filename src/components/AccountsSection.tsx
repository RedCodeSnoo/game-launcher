// Comptes en ligne (F-PAR-01) : Steam (clé API + SteamID) et SteamGridDB (jaquettes).
// Les clés sont envoyées au backend qui les range dans le coffre Windows ; elles ne sont jamais relues ici.
import { useCallback, useEffect, useState } from "react";
import { CheckCircle2, KeyRound } from "lucide-react";
import { api } from "../api";
import { errorMessage } from "../i18n";
import type { Accounts } from "../types";
import { useToasts } from "./Toasts";
import { Button, Field, TextInput } from "./ui";

export function SteamAccountForm({ onSaved }: { onSaved?: (count: number) => void }) {
  const toasts = useToasts();
  const [accounts, setAccounts] = useState<Accounts | null>(null);
  const [steamId, setSteamId] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    const a = await api.getAccounts();
    setAccounts(a);
    setSteamId((cur) => cur || a.steamId || a.localSteamId || "");
  }, []);
  useEffect(() => void load().catch(() => {}), [load]);

  const save = async () => {
    setBusy(true);
    try {
      const count = await api.setSteamAccount(steamId.trim(), apiKey.trim() || undefined);
      toasts.success(`Compte Steam connecté : ${count} jeux possédés`);
      setApiKey("");
      await load();
      onSaved?.(count);
    } catch (e) {
      toasts.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const configured = Boolean(accounts?.steamId && accounts.steamKeySet);
  return (
    <form
      className="flex flex-col gap-3"
      onSubmit={(e) => {
        e.preventDefault();
        void save();
      }}
    >
      {configured && (
        <p className="flex items-center gap-2 text-sm text-ok">
          <CheckCircle2 size={16} /> Compte Steam connecté (SteamID {accounts?.steamId})
        </p>
      )}
      <Field
        label="SteamID64 ou adresse du profil"
        hint={accounts?.localSteamId && !accounts.steamId ? "Pré-rempli avec le compte Steam connecté sur ce PC." : "17 chiffres, visible dans l'adresse de votre profil Steam."}
      >
        <TextInput value={steamId} onChange={(e) => setSteamId(e.target.value)} placeholder="7656119…" />
      </Field>
      <Field
        label="Clé API Steam Web"
        hint="Gratuite, à créer sur steamcommunity.com/dev/apikey. Votre profil et les « Détails des jeux » doivent être publics. La clé est conservée dans le coffre Windows."
      >
        <TextInput
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          type="password"
          autoComplete="off"
          placeholder={accounts?.steamKeySet ? "•••••••• (enregistrée – laisser vide pour la conserver)" : "32 caractères"}
        />
      </Field>
      <div className="flex gap-2">
        <Button variant="primary" type="submit" disabled={busy || !steamId.trim() || (!apiKey.trim() && !accounts?.steamKeySet)}>
          <KeyRound size={15} /> {busy ? "Vérification…" : configured ? "Mettre à jour" : "Connecter Steam"}
        </Button>
        {configured && (
          <Button
            type="button"
            variant="danger"
            onClick={async () => {
              await api.clearSteamAccount();
              toasts.info("Compte Steam déconnecté. Les données déjà synchronisées sont conservées.");
              setSteamId("");
              await load();
            }}
          >
            Déconnecter
          </Button>
        )}
      </div>
    </form>
  );
}

export function SgdbForm() {
  const toasts = useToasts();
  const [isSet, setIsSet] = useState(false);
  const [apiKey, setApiKey] = useState("");
  const [busy, setBusy] = useState(false);
  const load = useCallback(() => api.getAccounts().then((a) => setIsSet(a.sgdbKeySet)), []);
  useEffect(() => void load().catch(() => {}), [load]);

  const save = async (key?: string) => {
    setBusy(true);
    try {
      await api.setSgdbKey(key);
      toasts.success(key ? "Clé SteamGridDB enregistrée : les jaquettes manquantes vont être récupérées" : "Clé SteamGridDB supprimée");
      setApiKey("");
      await load();
    } catch (e) {
      toasts.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <form
      className="flex flex-col gap-3"
      onSubmit={(e) => {
        e.preventDefault();
        void save(apiKey.trim());
      }}
    >
      {isSet && (
        <p className="flex items-center gap-2 text-sm text-ok">
          <CheckCircle2 size={16} /> Clé SteamGridDB enregistrée
        </p>
      )}
      <Field label="Clé API SteamGridDB (facultative)" hint="Fournit les jaquettes des jeux hors Steam. À créer sur steamgriddb.com (Préférences > API).">
        <TextInput value={apiKey} onChange={(e) => setApiKey(e.target.value)} type="password" autoComplete="off" placeholder={isSet ? "•••••••• (enregistrée)" : ""} />
      </Field>
      <div className="flex gap-2">
        <Button variant="primary" type="submit" disabled={busy || !apiKey.trim()}>
          {busy ? "Vérification…" : "Enregistrer"}
        </Button>
        {isSet && (
          <Button type="button" variant="danger" onClick={() => save(undefined)} disabled={busy}>
            Supprimer la clé
          </Button>
        )}
      </div>
    </form>
  );
}
