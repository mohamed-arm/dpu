//! Manage transport session.

use crate::tcp;
use crate::tls;
#[cfg(feature = "initiator")]
use crate::tls_server;
#[cfg(feature = "responder")]
use crate::tls_client;
use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use log::info;
use mbedtls::ssl::Context;
use mbedtls_sys::psa::key_handle_t;
use parsec_client::BasicClient;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, net::TcpStream, fmt::Debug, sync::{Mutex, atomic::{AtomicU32, Ordering}, Arc}};

use std::time::SystemTime;


////////////////////////////////////////////////////////////////////////////////
// Various bits of persistent state.
////////////////////////////////////////////////////////////////////////////////
lazy_static! {
    /// Hashmap of session IDs (handles) mapped to sessions
    /// TODO: Remove expired sessions from hashmap
    /// XXX: Do we really need a session ID?
    static ref SESSIONS: Mutex<HashMap<SessionId, Session>> =
        Mutex::new(HashMap::new());
    static ref SESSION_COUNTER: AtomicU32 = AtomicU32::new(0);
}

pub type SessionId = u32;

#[cfg(feature = "responder")]
#[allow(dead_code)]
pub struct ResponderContext {
    key_handle: key_handle_t,
    client_attestation_type_list: [u16; 3],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EncryptionMode {
    Tls,
    Plaintext
}

/// Session
pub struct Session {
    /// TLS session. Exposes a transparent I/O abstraction that simplifies the use of TLS: just read/write from/to it
    tls_context: Context<TcpStream>,
    /// Encryption mode. For performance reasons local connections (e.g. between host and DPU) should be unencrypted while remote connections (e.g. between two DPUs) should be encrypted.
    /// Defaults to TLS
    encryption_mode: EncryptionMode,
    /// Additional context for miscellaneous responder-side data that must live through the entire session
    #[cfg(feature = "responder")]
    #[allow(dead_code)]
    responder_context: Option<ResponderContext>,
}

impl Session {
    /// Create session from responder's URL. Used by the initiator to attest the responder and establish a secure channel with the responder.
    /// A few notes on the implementation of attested TLS (https://github.com/CCC-Attestation/attested-tls-po) used here:
    ///   - The TLS client is the attester (responder here) and the TLS server is the relying party (initiator here)
    ///   - It only implements the background check attestation model
    ///   - Mutual attestation is not supported
    #[cfg(feature = "initiator")]
    pub fn from_url(responder_url: &str) -> Result<SessionId> {
        // TODO: Return session ID if session already exists

        // Connect to responder
        let socket = TcpStream::connect(responder_url)
            .map_err(|e| anyhow!("Could not connect to responder on {}: {}", responder_url, e))?;
        info!("Connected to responder on {}.", responder_url);

        info!("Initializing Veraison session...");
        tls_server::init_veraison_session("http://vfe:8080", 8);

        info!("Establishing TLS server context...");
        let config = tls_server::generate_tls_server_config()?;
        let mut tls_context = Context::new(Arc::new(config));
        tls_context.establish(socket, None)?;
        info!("TLS server context established");

        // Add session to hashmap
        let session_id = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
        SESSIONS
            .lock()
            .map_err(|_| anyhow!("Could not lock session hash table"))?
            .insert(
                session_id,
                Self {
                    tls_context,
                    encryption_mode: EncryptionMode::Tls,
                    responder_context: None,
                }
            );
        info!("Session added to hashmap");

        Ok(session_id)
    }

    /// Create session from socket. Used by the responder to get attested and
    /// establish a secure channel with the initiator. Cf. `Session::from_url()`
    /// for more details
    #[cfg(feature = "responder")]
    pub fn from_socket(socket: TcpStream) -> Result<SessionId> {
        // TODO: Return session ID if session already exists

        // Establish TLS client context
        info!("Establishing TLS client context...");
        let mut time = SystemTime::now(); 
        let (config, key_handle, client_attestation_type_list) = tls_client::generate_tls_client_config()?;
println!("===  {} [DPU-RM] generate_tls_client_config ({:?}):", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros(),  SystemTime::now().duration_since(time).unwrap());

        let mut time = SystemTime::now(); 
        let mut tls_context = Context::new(Arc::new(config));
println!("===  {} [DPU-RM] Context::new ({:?}):", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros(),  SystemTime::now().duration_since(time).unwrap());

        let mut time = SystemTime::now(); 
        tls_context.establish(socket, None)?;
println!("===  {} [DPU-RM] establish({:?}):", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros(),  SystemTime::now().duration_since(time).unwrap());
        info!("TLS client context established");


        // Remove PARSEC key to avoid `PSA_ERROR_ALREADY_EXISTS` error next time we establish a TLS context using the same PARSEC service
        // TODO: grab the PARSEC client instance (PARSEC_BASIC_CLIENT) already generated by the PARSEC SE driver
        // TODO: find a better way to get the key's name
        // TODO: also destroy key when destroying session
        let mut time = SystemTime::now(); 
        let mut client = BasicClient::new_naked()?;
        client.set_default_auth(Some(String::from("Parsec SE Driver")))?;
        client.set_default_provider()?;
        let _ = client.psa_destroy_key("parsec-se-driver-key48879");

        // Add session to hashmap
        let session_id = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
        SESSIONS
            .lock()
            .map_err(|_| anyhow!("Could not lock session hash table"))?
            .insert(
                session_id,
                Self {
                    tls_context,
                    encryption_mode: EncryptionMode::Tls,
                    responder_context: Some(ResponderContext {
                        key_handle: *key_handle,
                        client_attestation_type_list: *client_attestation_type_list,
                    }),
                }
            );
println!("===  {} [DPU-RM] post-establish({:?}):", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros(),  SystemTime::now().duration_since(time).unwrap());

        Ok(session_id)
    }

    pub fn set_encryption_mode(session_id: SessionId, encryption_mode: EncryptionMode) -> Result<()> {
        let mut s = SESSIONS
            .lock()
            .map_err(|_| anyhow!("Could not lock session table"))?;
        let s = s
            .get_mut(&session_id)
            .ok_or(anyhow!("Session does not exist"))?;
        s.encryption_mode = encryption_mode;
        Ok(())
    }

    /// Send application message
    pub fn send_message<T>(session_id: SessionId, data: T) -> Result<()>
    where
    T: Serialize + Debug,
    {
        let mut s = SESSIONS
            .lock()
            .map_err(|_| anyhow!("Could not lock session table"))?;
        let s = s
            .get_mut(&session_id)
            .ok_or(anyhow!("Session does not exist"))?;
        match s.encryption_mode {
            EncryptionMode::Tls => tls::send_message(&mut s.tls_context, data),
            EncryptionMode::Plaintext => tcp::send_message(
                s
                    .tls_context
                    .io_mut()
                    .ok_or(anyhow!("Context has no valid I/O"))?,
                data
            ),
        }
    }

    /// Receive application message
    pub fn receive_message<T>(session_id: SessionId) -> Result<T>
    where
    T: DeserializeOwned + Debug,
    {
        let mut s = SESSIONS
            .lock()
            .map_err(|_| anyhow!("Could not lock session table"))?;
        let s = s
            .get_mut(&session_id)
            .ok_or(anyhow!("Session does not exist"))?;
        match s.encryption_mode {
            EncryptionMode::Tls => tls::receive_message(&mut s.tls_context),
            EncryptionMode::Plaintext => tcp::receive_message(
                s
                    .tls_context
                    .io_mut()
                    .ok_or(anyhow!("Context has no valid I/O"))?
            ),
        }
    }
}
