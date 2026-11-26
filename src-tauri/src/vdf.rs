use std::collections::BTreeMap;

/// Value-Key de Valve (libraryfolders.vdf, appmanifest.acf, localconfig.vdf).
#[derive(Debug, Clone)]
pub enum Node {
    Value(String),
    Map(Map),
}

#[derive(Debug, Clone, Default)]
pub struct Map(pub BTreeMap<String, Node>);

impl Node {
    pub fn map(&self, key: &str) -> Option<&Map> {
        match self {
            Node::Map(map) => map.map(key),
            Node::Value(_) => None,
        }
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        match self {
            Node::Map(map) => map.str(key),
            Node::Value(_) => None,
        }
    }
}

impl Map {
    pub fn get(&self, key: &str) -> Option<&Node> {
        self.0.get(key)
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Some(Node::Value(value)) => Some(value),
            _ => None,
        }
    }

    pub fn u64(&self, key: &str) -> Option<u64> {
        self.str(key).and_then(|value| value.parse().ok())
    }

    pub fn map(&self, key: &str) -> Option<&Map> {
        match self.get(key) {
            Some(Node::Map(map)) => Some(map),
            _ => None,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Node)> {
        self.0.iter()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Str(String),
    Open,
    Close,
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => out.push(Token::Open),
            '}' => out.push(Token::Close),
            '"' => {
                let mut value = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => {
                            if let Some(escaped) = chars.next() {
                                value.push(escaped);
                            }
                        }
                        '"' => break,
                        _ => value.push(c),
                    }
                }
                out.push(Token::Str(value));
            }
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => {}
            c => {
                let mut value = String::from(c);
                while let Some(&next) = chars.peek() {
                    if next.is_whitespace() || next == '"' || next == '{' || next == '}' {
                        break;
                    }
                    value.push(next);
                    chars.next();
                }
                out.push(Token::Str(value));
            }
        }
    }
    out
}

pub fn parse(input: &str) -> Option<Node> {
    let tokens = tokenize(input);
    let mut index = 0;
    let map = parse_map(&tokens, &mut index)?;
    Some(Node::Map(map))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACF: &str = r#"
        "AppState"
        {
            "appid"    "1091500"
            "name"     "Cyberpunk 2077"
            "installdir" "Cyberpunk 2077"
            "SizeOnDisk" "73391594496"
            "UserConfig" { "language" "spanish" }
        }
    "#;

    #[test]
    fn parsea_appmanifest() {
        let node = parse(ACF).expect("vdf válido");
        let state = node.map("AppState").expect("AppState");
        assert_eq!(state.str("appid"), Some("1091500"));
        assert_eq!(state.str("name"), Some("Cyberpunk 2077"));
        assert_eq!(state.u64("SizeOnDisk"), Some(73_391_594_496));
        assert_eq!(
            state.map("UserConfig").and_then(|c| c.str("language")),
            Some("spanish")
        );
    }
}

fn parse_map(tokens: &[Token], index: &mut usize) -> Option<Map> {
    let mut map = Map::default();
    while *index < tokens.len() {
        match &tokens[*index] {
            Token::Close => {
                *index += 1;
                return Some(map);
            }
            Token::Open => {
                // Bloque sin clave: se ignora su contenido.
                *index += 1;
                parse_map(tokens, index)?;
            }
            Token::Str(key) => {
                let key = key.clone();
                *index += 1;
                match tokens.get(*index)? {
                    Token::Str(value) => {
                        map.0.insert(key, Node::Value(value.clone()));
                        *index += 1;
                    }
                    Token::Open => {
                        *index += 1;
                        let child = parse_map(tokens, index)?;
                        map.0.insert(key, Node::Map(child));
                    }
                    Token::Close => return None,
                }
            }
        }
    }
    Some(map)
}
