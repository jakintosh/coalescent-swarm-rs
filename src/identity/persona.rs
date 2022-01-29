use std::collections::HashMap;

#[derive(Clone)]
pub struct Persona {
    pub root: Vec<u8>,
    pub keys: HashMap<String, String>,
}
