//! Discord Rich Presence (V1.5, option) : « Joue à <jeu> », avec la jaquette et le temps écoulé.
//!
//! Protocole IPC local de Discord : tube nommé `\\.\pipe\discord-ipc-N`, trames `[opcode u32 LE][longueur u32 LE][JSON]`.
//! Il faut l'identifiant d'une « application » Discord (créée gratuitement par l'utilisateur sur
//! discord.com/developers) : c'est son nom qui s'affiche (« Joue à GameLauncher »).
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

pub const OP_HANDSHAKE: u32 = 0;
pub const OP_FRAME: u32 = 1;
pub const OP_CLOSE: u32 = 2;

pub fn encode(op: u32, payload: &Value) -> Vec<u8> {
    let body = payload.to_string().into_bytes();
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&op.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

pub fn decode(bytes: &[u8]) -> Option<(u32, Value)> {
    if bytes.len() < 8 {
        return None;
    }
    let op = u32::from_le_bytes(bytes[0..4].try_into().ok()?);
    let len = u32::from_le_bytes(bytes[4..8].try_into().ok()?) as usize;
    let body = bytes.get(8..8 + len)?;
    Some((op, serde_json::from_slice(body).ok()?))
}

/// Activité affichée : titre du jeu, début de la session, image (URL https publique) facultative.
pub fn activity(game: &str, started_at: i64, image: Option<&str>, app_name: &str) -> Value {
    let mut assets = json!({ "large_text": game });
    if let Some(url) = image.filter(|u| u.starts_with("https://")) {
        assets["large_image"] = json!(url);
    }
    json!({
        "details": game,
        "state": format!("via {app_name}"),
        "timestamps": { "start": started_at },
        "assets": assets,
        "instance": false,
    })
}

pub struct DiscordIpc {
    pipe: File,
}

impl DiscordIpc {
    /// Connexion au client Discord local (le premier tube disponible) et poignée de main.
    pub fn connect(client_id: &str) -> Result<Self> {
        if client_id.is_empty() || !client_id.chars().all(|c| c.is_ascii_digit()) {
            bail!("identifiant d'application Discord invalide");
        }
        let pipe = (0..10)
            .find_map(|i| OpenOptions::new().read(true).write(true).open(format!(r"\\.\pipe\discord-ipc-{i}")).ok())
            .context("Discord n'est pas lancé")?;
        let mut ipc = DiscordIpc { pipe };
        ipc.send(OP_HANDSHAKE, &json!({ "v": 1, "client_id": client_id }))?;
        let (op, reply) = ipc.read()?;
        if op == OP_CLOSE || reply.get("evt").and_then(|e| e.as_str()) != Some("READY") {
            bail!("Discord a refusé la connexion : {}", reply.get("message").and_then(|m| m.as_str()).unwrap_or("réponse inattendue"));
        }
        Ok(ipc)
    }

    fn send(&mut self, op: u32, payload: &Value) -> Result<()> {
        self.pipe.write_all(&encode(op, payload))?;
        self.pipe.flush()?;
        Ok(())
    }

    fn read(&mut self) -> Result<(u32, Value)> {
        let mut header = [0u8; 8];
        self.pipe.read_exact(&mut header)?;
        let len = u32::from_le_bytes(header[4..8].try_into()?) as usize;
        if len > 1 << 20 {
            bail!("trame Discord trop grande");
        }
        let mut body = vec![0u8; len];
        self.pipe.read_exact(&mut body)?;
        Ok((u32::from_le_bytes(header[0..4].try_into()?), serde_json::from_slice(&body).unwrap_or(Value::Null)))
    }

    /// `None` efface l'activité.
    pub fn set_activity(&mut self, activity: Option<Value>) -> Result<()> {
        let payload = json!({
            "cmd": "SET_ACTIVITY",
            "args": { "pid": std::process::id(), "activity": activity },
            "nonce": uuid::Uuid::new_v4().to_string(),
        });
        self.send(OP_FRAME, &payload)?;
        let (_, reply) = self.read()?;
        if reply.get("evt").and_then(|e| e.as_str()) == Some("ERROR") {
            bail!("Discord : {}", reply.pointer("/data/message").and_then(|m| m.as_str()).unwrap_or("erreur"));
        }
        Ok(())
    }
}

/// Image d'un jeu utilisable par Discord : la jaquette publique du CDN Steam (les jaquettes locales ne sont pas
/// accessibles depuis Discord).
pub fn public_cover(source: crate::models::SourceId, external_id: &str) -> Option<String> {
    (source == crate::models::SourceId::Steam && external_id.chars().all(|c| c.is_ascii_digit())).then(|| format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{external_id}/library_600x900.jpg"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_roundtrip() {
        let frame = encode(OP_HANDSHAKE, &json!({ "v": 1, "client_id": "123" }));
        assert_eq!(&frame[0..4], &0u32.to_le_bytes());
        assert_eq!(u32::from_le_bytes(frame[4..8].try_into().unwrap()) as usize, frame.len() - 8);
        assert_eq!(decode(&frame), Some((0, json!({ "v": 1, "client_id": "123" }))));
        assert_eq!(decode(&frame[..6]), None);
    }

    #[test]
    fn activity_payload() {
        let a = activity("Hades", 1_800_000_000, Some("https://cdn.cloudflare.steamstatic.com/steam/apps/1145360/library_600x900.jpg"), "GameLauncher");
        assert_eq!(a["details"], "Hades");
        assert_eq!(a["state"], "via GameLauncher");
        assert_eq!(a["timestamps"]["start"], 1_800_000_000);
        assert!(a["assets"]["large_image"].as_str().unwrap().starts_with("https://"));
        let local = activity("Jeu local", 1, Some("C:\\images\\x.jpg"), "GameLauncher");
        assert!(local["assets"].get("large_image").is_none(), "jamais de chemin local");
        assert_eq!(public_cover(crate::models::SourceId::Steam, "1145360").as_deref(), Some("https://cdn.cloudflare.steamstatic.com/steam/apps/1145360/library_600x900.jpg"));
        assert_eq!(public_cover(crate::models::SourceId::Riot, "valorant.live"), None);
        assert!(DiscordIpc::connect("abc").is_err());
    }
}
