#![allow(unused_variables)]
pub use agent::Agent;
pub use persona::Persona;
use serde::{Deserialize, Serialize};

mod agent;
mod persona;

use crate::data::HashId;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct KeyPair {
    pub public_key: HashId,
    private_key: HashId,
}

impl KeyPair {
    pub fn generate() -> KeyPair {
        KeyPair {
            public_key: HashId([0u8; 32]),
            private_key: HashId([0u8; 32]),
        }
    }

    pub fn sign(&self, bytes: impl AsRef<[u8]>) -> Vec<u8> {
        // stub, just returns the same bytes for now
        bytes.as_ref().to_owned()
    }
    pub fn verify(&self, bytes: impl AsRef<[u8]>, signature: impl AsRef<[u8]>) -> bool {
        // stub, just returns true for now
        true
    }

    pub fn encrypt(&self, bytes: impl AsRef<[u8]>) -> Vec<u8> {
        // stub, just returns the same bytes for now
        bytes.as_ref().to_owned()
    }
    pub fn decrypt(&self, bytes: impl AsRef<[u8]>) -> Vec<u8> {
        // stub, just returns the same bytes for now
        bytes.as_ref().to_owned()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Identity {
    pub encryption_keys: KeyPair,
    pub signing_keys: KeyPair,
}

impl Identity {
    pub fn generate() -> Identity {
        let encryption_keys = KeyPair::generate();
        let signing_keys = KeyPair::generate();

        Identity {
            encryption_keys,
            signing_keys,
        }
    }
}
