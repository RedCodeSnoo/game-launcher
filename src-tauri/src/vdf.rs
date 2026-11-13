//! Parseur du format texte VDF / KeyValues de Valve (libraryfolders.vdf, appmanifest_*.acf).
use anyhow::{anyhow, bail, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Vdf {
    Str(String),
    Obj(Vec<(String, Vdf)>),
}

impl Vdf {
    /// Recherche d'une clé (insensible à la casse, comme Steam).
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Vdf::Obj(items) => items.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v),
            Vdf::Str(_) => None,
        }
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Some(Vdf::Str(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn entries(&self) -> &[(String, Vdf)] {
        match self {
            Vdf::Obj(items) => items,
            Vdf::Str(_) => &[],
        }
    }
}

#[derive(Debug, PartialEq)]
enum Token {
    Str(String),
    Open,
    Close,
}

fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            c if c.is_whitespace() || c == '\u{feff}' => {
                chars.next();
            }
            '/' => {
                chars.next();
                if chars.peek() == Some(&'/') {
                    for c in chars.by_ref() {
                        if c == '\n' {
                            break;
                        }
                    }
                } else {
                    bail!("caractère '/' inattendu");
                }
            }
            '{' => {
                chars.next();
                tokens.push(Token::Open);
            }
            '}' => {
                chars.next();
                tokens.push(Token::Close);
            }
            '"' => {
                chars.next();
                let mut s = String::new();
                loop {
                    match chars.next() {
                        None => bail!("chaîne non terminée"),
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(other) => s.push(other),
                            None => bail!("échappement non terminé"),
                        },
                        Some(other) => s.push(other),
                    }
                }
                tokens.push(Token::Str(s));
            }
            _ => {
                // Jeton non entouré de guillemets.
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || c == '{' || c == '}' || c == '"' {
                        break;
                    }
                    s.push(c);
                    chars.next();
                }
                tokens.push(Token::Str(s));
            }
        }
    }
    Ok(tokens)
}

fn parse_object(tokens: &[Token], pos: &mut usize, top: bool) -> Result<Vdf> {
    let mut items = Vec::new();
    loop {
        match tokens.get(*pos) {
            None => {
                if top {
                    return Ok(Vdf::Obj(items));
                }
                bail!("accolade fermante manquante");
            }
            Some(Token::Close) => {
                if top {
                    bail!("accolade fermante inattendue");
                }
                *pos += 1;
                return Ok(Vdf::Obj(items));
            }
            Some(Token::Open) => bail!("accolade ouvrante sans clé"),
            Some(Token::Str(key)) => {
                *pos += 1;
                match tokens.get(*pos) {
                    Some(Token::Str(value)) => {
                        *pos += 1;
                        items.push((key.clone(), Vdf::Str(value.clone())));
                    }
                    Some(Token::Open) => {
                        *pos += 1;
                        let child = parse_object(tokens, pos, false)?;
                        items.push((key.clone(), child));
                    }
                    _ => bail!("valeur manquante pour la clé « {key} »"),
                }
            }
        }
    }
}

/// Analyse un document VDF. Le résultat est un objet contenant la ou les clés racines.
pub fn parse(input: &str) -> Result<Vdf> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err(anyhow!("document VDF vide"));
    }
    let mut pos = 0;
    parse_object(&tokens, &mut pos, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_objects_and_escapes() {
        let doc = "\"root\"\n{\n\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t// commentaire\n\t\"apps\"\n\t{\n\t\t\"10\"\t\"123\"\n\t}\n}\n";
        let v = parse(doc).unwrap();
        let root = v.get("root").unwrap();
        assert_eq!(root.str("path"), Some("C:\\Program Files (x86)\\Steam"));
        assert_eq!(root.get("apps").unwrap().str("10"), Some("123"));
    }

    #[test]
    fn keys_are_case_insensitive() {
        let v = parse("\"AppState\" { \"appid\" \"5\" }").unwrap();
        assert_eq!(v.get("appstate").unwrap().str("AppID"), Some("5"));
    }

    #[test]
    fn rejects_broken_documents() {
        assert!(parse("").is_err());
        assert!(parse("\"a\" {").is_err());
        assert!(parse("\"a\" \"unterminated").is_err());
        assert!(parse("}").is_err());
    }
}
