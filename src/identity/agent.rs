use super::{Identity, Persona};
use crate::data::HashId;
use std::collections::HashMap;

pub struct Agent {
    pub id: HashId,
    pub identity: Identity,
    pub persona: Persona,
}

impl Agent {
    pub fn create_with_new_persona() -> (Agent, Persona) {
        let identity = Identity::generate();
        let root = identity
            .signing_keys
            .sign(&identity.signing_keys.public_key);

        let persona = Persona {
            root,
            keys: HashMap::new(),
        };

        let agent_hash = Agent::derive_agent_hash(&identity, &persona);
        let agent = Agent {
            id: agent_hash.into(),
            identity,
            persona: persona.clone(),
        };

        (agent, persona)
    }

    pub fn create_from_persona(persona: Persona) -> Agent {
        let identity = Identity::generate();
        let agent_hash = Agent::derive_agent_hash(&identity, &persona);
        Agent {
            id: agent_hash.into(),
            identity,
            persona,
        }
    }

    fn derive_agent_hash(identity: &Identity, persona: &Persona) -> [u8; 32] {
        let signing_key_bytes: &[u8] = identity.signing_keys.public_key.as_ref();
        let encryption_key_bytes: &[u8] = identity.encryption_keys.public_key.as_ref();

        let mut agent_id_bytes: Vec<u8> = Vec::new();
        agent_id_bytes.extend(signing_key_bytes);
        agent_id_bytes.extend(encryption_key_bytes);
        agent_id_bytes.extend(&persona.root);

        let params = blake2s_simd::Params::new();
        let hash = params.hash(&agent_id_bytes);

        *hash.as_array()
    }
}
