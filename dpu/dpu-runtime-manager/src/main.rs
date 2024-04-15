//! DPU-specific material for the Runtime Manager enclave
//!
//! NB: note that the attestation flow presented in this
//! module is *completely* insecure and just presented here as a
//! mockup of what a real attestation flow should look like.  See
//! the AWS Nitro Enclave attestation flow for a real example.
//!
//! ## Authors
//!
//! The Veracruz Development Team.
//!
//! ## Licensing and copyright notice
//!
//! See the `LICENSE.md` file in the Veracruz root directory for
//! information on licensing and copyright.

use anyhow::{anyhow, Result};
use clap::Arg;
use hex::decode_to_slice;
use lazy_static::lazy_static;
use log::{debug, error, info};
use transport::session::Session;
use std::{net::TcpListener, sync::Mutex};
use std::time::SystemTime;

mod dpu_runtime;

lazy_static! {
    static ref RUNTIME_MANAGER_MEASUREMENT: Mutex<Vec<u8>> = Mutex::new(vec![0u8; 32]);
}

/// Default values for use by the DPU.
const DEFAULT_LISTENING_ADDRESS: &str = "127.0.0.1";
const DEFAULT_LISTENING_PORT: &str = "6666";

////////////////////////////////////////////////////////////////////////////////
// Entry point and message dispatcher.
////////////////////////////////////////////////////////////////////////////////

fn main() -> Result<(), String> {
    dpu_main().map_err(|err| {
        format!(
            "DPU Runtime Manager::main encap returned error:{:?}",
            err
        )
    })
}

/// Main entry point for DPU: parses command line arguments to find the port
/// number we should be listening on for incoming connections from the Veracruz
/// server.  Parses incoming messages, and acts on them.
pub fn dpu_main() -> Result<()> {
    env_logger::init();

    let matches = clap::Command::new("DPU runtime manager enclave")
        .author("The Veracruz Development Team")
        .arg(
            Arg::new("address")
                .short('a')
                .long("address")
                .num_args(1)
                .required(false)
                .help("Listening address.")
                .value_name("ADDRESS"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .num_args(1)
                .required(false)
                .help("Listening port.")
                .value_name("PORT"),
        )
        .arg(
            Arg::new("runtime_manager_measurement")
                .short('m')
                .long("measurement")
                .num_args(1)
                .required(true)
                .help("SHA256 measurement of the Runtime Manager enclave binary.")
                .value_name("MEASUREMENT"),
        )
        .get_matches();

    let listening_address = match matches.get_one::<String>("address") {
        Some(s) => s.as_str(),
        None => DEFAULT_LISTENING_ADDRESS,
    };

    let listening_port = match matches.get_one::<String>("port") {
        Some(s) => s.as_str(),
        None => DEFAULT_LISTENING_PORT,
    };

    let measurement =
        if let Some(measurement) = matches.get_one::<String>("runtime_manager_measurement") {
            measurement
        } else {
            error!("No measurement given. Exiting...");
            return Err(anyhow!("RuntimeManagerError::CommandLineArguments"));
        };

    let mut measurement_bytes = vec![0u8; 32];

    if let Err(err) = decode_to_slice(measurement, &mut measurement_bytes) {
        error!(
            "Failed to decode Runtime Manager measurement ({}).  Error produced: {:?}.",
            measurement, err
        );
        return Err(anyhow!("RuntimeManagerError::CommandLineArguments"));
    }

    {
        let mut rmm = RUNTIME_MANAGER_MEASUREMENT.lock().unwrap();
        *rmm = measurement_bytes;
    }

    let dpu_runtime: dpu_runtime::DPURuntime = dpu_runtime::DPURuntime::new()?;

    let address = format!("{}:{}", listening_address, listening_port);
    let listener = TcpListener::bind(&address).map_err(|e| {
        anyhow!("Could not bind TCP listener: {}", e)
    })?;
    info!("TCP listener created on {}.", address);

    debug!("dpu_runtime_manager::dpu_main accept succeeded. looping");
    // We handle connections one at a time: once a connection is accepted,
    // it is listened to till the end.
    // Note that the runtime manager may synchronously handle several
    // connections, for instance when receiving a message from A to be relayed
    // to B: the message is sent to B while A is still waiting for a result
    // TODO: multithread this
    loop {
        let (runtime_manager_socket, _) = listener.accept().map_err(|ioerr| {
            anyhow!(
                "Failed to accept any incoming TCP connection.  Error produced: {}.",
                ioerr
            )
        })?;
        info!("Accepted connection from {:?}.", runtime_manager_socket);

        let mut time = SystemTime::now();

        // Establish secure channel and save session to hashmap
        let session_id = Session::from_socket(runtime_manager_socket)?;
        println!("===  {} [DPU-RM] from_socket ({:?}):", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros(),  SystemTime::now().duration_since(time).unwrap());

        debug!("DPU Runtime Manager::main accept succeeded. Looping");
        loop {
            // Receive requests and serve them. Terminate connection if
            // processing fails, e.g. if receiving an invalid message
            if dpu_runtime.decode_dispatch(session_id).is_err() {
                break;
            }
        }
    }
}
