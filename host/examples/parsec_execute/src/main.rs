//! Provision and execute parsec.

use anyhow::anyhow;
use log::{error, info};
use utils::attestation;
use transport::{messages::{Request, Response, Status}, session::Session};

use std::fs;
use std::fs::File;
use std::io::Read;

const DPU_SERVER_URL: &str = "127.0.0.1:6666";
const ATTESTATION_SERVER_URL: &str = "127.0.0.1:3010";

const PARSEC_APP_NAME: &str = "parsec-tool";
const PARSEC_PERM_CMD: &str = "chmod +x parsec-tool";
const PARSEC_EXECUTE_CMD: &str = "export PARSEC_SERVICE_CONN_TYPE=tcp; export PARSEC_SERVICE_CONN_IP_ADDR=172.28.97.222; export PARSEC_SERVICE_CONN_PORT_NO=8002 ; ./parsec-tool ping";

// Read program file
fn get_file_data(filename: &String) -> anyhow::Result<Vec<u8>> {

    let mut file = File::open(&filename).unwrap();
    let metadata = fs::metadata(&filename).unwrap();
    let mut file_content = vec![0; metadata.len() as usize];
    file.read(&mut file_content).unwrap();

    Ok(file_content)
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    // TODO: Parse arguments with clap
    let dpu_server_url = DPU_SERVER_URL;
    let proxy_attestation_server_url = ATTESTATION_SERVER_URL;

    let dpu_session_id = Session::from_url(dpu_server_url)?;

    info!("Attesting DPU...");
    attestation::request_attestation(dpu_session_id, proxy_attestation_server_url)?;
    info!("Successfully attested DPU.");

    let filename = PARSEC_APP_NAME.to_owned();
    let data = get_file_data(&filename).unwrap();

    info!("Provisioning DPU...");
    Session::send_message(dpu_session_id, &Request::UploadFile(filename, data))
        .map_err(|e| {
            error!("Failed to send provisioning message to DPU.  Error returned: {:?}.", e);
            e
        })?;
    let response = Session::receive_message(dpu_session_id).map_err(|e| {
        error!("Failed to receive response to provisioning message.  Error received: {:?}.", e);
        e
    })?;
    match response {
        Response::Status(Status::Success(_)) => {
            info!("Successfully provisioned DPU.");
        },
        _ => {
            error!("Error provisioning DPU.");
            return Err(anyhow!("Provisioning failure."));
        },
    }

    info!("Requesting permission change...");
    Session::send_message(dpu_session_id, &Request::Execute(PARSEC_PERM_CMD.to_owned()))
        .map_err(|e| {
            error!("Failed to send execution message to DPU.  Error returned: {:?}.", e);
            e
        })?;
    let response = Session::receive_message(dpu_session_id).map_err(|e| {
        error!("Failed to receive response to execution message.  Error received: {:?}.", e);
        e
    })?;
    match response {
        Response::Status(Status::Success(output)) => {
            info!("Successfully executed code remotely. Output: \n{}", output);
        },
        _ => {
            error!("Error executing code remotely.");
            return Err(anyhow!("Remote execution failure."));
        },
    }

    info!("Requesting remote execution...");
    Session::send_message(dpu_session_id, &Request::Execute(PARSEC_EXECUTE_CMD.to_owned()))
        .map_err(|e| {
            error!("Failed to send execution message to DPU.  Error returned: {:?}.", e);
            e
        })?;
    let response = Session::receive_message(dpu_session_id).map_err(|e| {
        error!("Failed to receive response to execution message.  Error received: {:?}.", e);
        e
    })?;
    match response {
        Response::Status(Status::Success(output)) => {
            info!("Successfully executed code remotely. Output: \n{}", output);
        },
        _ => {
            error!("Error executing code remotely.");
            return Err(anyhow!("Remote execution failure."));
        },
    }

    Ok(())
}
