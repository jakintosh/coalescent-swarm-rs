use std::collections::HashMap;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Persona {
    pub root: Vec<u8>,
    pub keys: HashMap<String, String>,
}
