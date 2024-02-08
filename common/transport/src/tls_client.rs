//! TLS client

use crate::tls;
use anyhow::Result;
use mbedtls::{ssl::{config::{Endpoint, Preset, Transport}, Config, Version}, rng, x509::Certificate};
use mbedtls_sys::*;
use mbedtls_sys::psa::*;
use parsec_se_driver::PARSEC_SE_DRIVER;
use std::borrow::Cow;
use std::sync::Arc;

pub fn generate_tls_client_config() -> Result<(Config, Box<key_handle_t>, Box<[u16; 3]>)> {
    // Register Parsec SE driver. Must be done before `psa_crypto_init()` triggered by `Config::new()`
    let location: key_location_t = 0x000001;
    let parsec_se_driver = unsafe { &PARSEC_SE_DRIVER as *const _ as *const drv_se_t };
    let ret;
    unsafe {
        ret = register_se_driver(location, parsec_se_driver);
    }
    if ret != 0 {
        panic!("Register failed (status = {})\n", ret);
    }

    let mut config = Config::new(Endpoint::Client, Transport::Stream, Preset::Default);

    config.set_min_version(Version::Tls13)?;
    config.set_max_version(Version::Tls13)?;

    let entropy = Arc::new(rng::OsEntropy::new());
    let rng = Arc::new(rng::CtrDrbg::new(entropy, None)?);
    config.set_rng(rng);

    let cert = Arc::new(Certificate::from_pem_multiple(tls::ROOT_CA_CERT.as_bytes())?);
    config.set_ca_list(cert, None);

    // Configure debugging
    let dbg_callback =
    |level: i32, file: Cow<'_, str>, line: i32, message: Cow<'_, str>| {
        print!("{} {}:{} {}", level, file, line, message);
    };
    config.set_dbg_callback(dbg_callback);
    unsafe { mbedtls::set_global_debug_threshold(5); }

    // `key_handle` and `client_attestation_type_list` must:
    //  1) be allocated on the heap, so that they may be accessed by Mbed TLS
    //     code later on
    //  2) be returned, so that they may be stored to live through the entire
    //     TLS session
    let mut key_handle = Box::new(0 as key_handle_t);
    let client_attestation_type_list = Box::new([
        TLS_ATTESTATION_TYPE_EAT as u16,
        TLS_ATTESTATION_TYPE_NONE as u16,
        TLS_ATTESTATION_TYPE_NONE as u16,
    ]);

    // Generate PSA key
    // Warning: Mind the dangling pointers below, Rust won't catch them!
    let key_pair_id: key_id_t = 0xBEEF;
    unsafe {
        ssl_conf_client_attestation_type(config.get_mut_inner(),client_attestation_type_list.as_ptr());
        let mut key_pair_attributes = key_attributes_init();
        set_key_id(&mut key_pair_attributes, key_pair_id);
        let lifetime = 0x000001 << 8 | 0x000001;
        set_key_lifetime(&mut key_pair_attributes, lifetime);
        set_key_usage_flags(&mut key_pair_attributes, KEY_USAGE_SIGN_HASH as u32 | KEY_USAGE_VERIFY_HASH as u32);
        set_key_algorithm(&mut key_pair_attributes, ALG_ECDSA_BASE as u32 | (ALG_SHA_256 as u32 & ALG_HASH_MASK as u32));
        set_key_type(&mut key_pair_attributes, KEY_TYPE_ECC_KEY_PAIR_BASE as u16 | ECC_FAMILY_SECP_R1 as u16);
        set_key_bits(&mut key_pair_attributes, 256);

        let ret = generate_key(&key_pair_attributes, key_handle.as_mut());
        if ret != 0 {
            panic!("psa_generate_key failed (status = {})\n", ret);
        }
        ssl_conf_client_rpk(config.get_mut_inner(), key_handle.as_mut());
    }

    Ok((config, key_handle, client_attestation_type_list))
}