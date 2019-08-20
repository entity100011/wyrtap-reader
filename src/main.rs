mod logger;

use crypto::aead::AeadDecryptor;
use crypto::aes::KeySize;
use crypto::aes_gcm::AesGcm;
use crypto::digest::Digest;
use crypto::sha3::Sha3;
use logger::Logger;
use openssl::derive::Deriver;
use openssl::pkey::PKey;
use std::env;
use std::fs;
use std::io::Write;
use std::process::Command;

fn main() {
    let mut logger = Logger::new();
    let args: Vec<String> = env::args().collect();
    let username = whoami::username();

    let (shared_secret, ciphertext, tag, iv) = match args.len() {
        n if n == 2 as usize => {
            // Read the WYR file and the encrypted private key file
            match (
                // Index 0 is the process name
                fs::read(args.get(1).expect("Failed...?")),
                fs::read(format!("/home/{}/private_wyrtap.pem", &username)),
            ) {
                (Ok(wyr), Ok(private_bytes)) => {
                    // Sort the WYR mess first
                    logger.info(format!("Length: {}", wyr.len())); // 710172
                    std::io::stdout().lock().flush().unwrap();
                    let ciphertext = wyr[0..wyr.len() - 296].to_vec();
                    let public_bytes = wyr[wyr.len() - 296..wyr.len() - 28].to_vec();
                    let iv = wyr[wyr.len() - 28..wyr.len() - 16].to_vec(); //710156
                    let tag = wyr[wyr.len() - 16..].to_vec();

                    // Get PKey from public_bytes
                    match PKey::public_key_from_pem(public_bytes.as_slice()) {
                        Ok(public_key) => {
                            // Prompt user for passphrase
                            let passphrase =
                                rpassword::read_password_from_tty(Some("Enter passphrase: "))
                                    .expect("Failed to read from stdin");

                            // Get PKey from private_bytes
                            match PKey::private_key_from_pem_passphrase(
                                private_bytes.as_slice(),
                                passphrase.as_bytes(),
                            ) {
                                Ok(private_key) => {
                                    // Create new Deriver
                                    match Deriver::new(&private_key) {
                                        Ok(mut deriver) => {
                                            match deriver.set_peer(&public_key) {
                                                Ok(_) => {
                                                    // Derive shared secret
                                                    match deriver.derive_to_vec() {
                                                        Ok(n) => (n, ciphertext, tag, iv),
                                                        Err(e) => {
                                                            logger.error(format!(
                                                                "Failed to derive shared secret: {}",
                                                                e
                                                            ));
                                                            return;
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    logger.error(format!(
                                                        "Failed to set peer for Deriver: {}",
                                                        e
                                                    ));
                                                    return;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            logger.error(format!(
                                                "Failed to create new Deriver: {}",
                                                e
                                            ));
                                            return;
                                        }
                                    }
                                }
                                Err(e) => {
                                    logger.error(format!("Failed to get private key: {}", e));
                                    return;
                                }
                            }
                        }
                        Err(e) => {
                            logger.error(format!("Failed to get public key from: {}", e));
                            return;
                        }
                    }
                }
                _ => {
                    logger.error("Failed to read one of the files");
                    return;
                }
            }
        }
        n => {
            logger.error(format!(
                "Expected 1 arguments (.xz.wyr file), found {} arguments",
                n - 1
            ));
            return;
        }
    };

    // We now have the shared secret, after all that mess of a code above
    // Decrypt WYR to get LZMA compressed PCM 24-bit audio
    let mut hasher = Sha3::sha3_256();
    hasher.input(shared_secret.as_slice());
    let mut key = [0u8; 32];
    hasher.result(&mut key);
    let mut aes_gcm = AesGcm::new(
        KeySize::KeySize256,
        &key,
        iv.as_slice(),
        Vec::new().as_slice(),
    );
    let mut plaintext = vec![0u8; ciphertext.len()];
    aes_gcm.decrypt(ciphertext.as_slice(), &mut plaintext, tag.as_slice());

    // We now have the LZMA compressed PCM 24-bit audio
    // Decompress it to just PCM 24-bit audio
    let audio = match lzma::decompress(plaintext.as_slice()) {
        Ok(n) => n,
        Err(e) => {
            logger.error(format!("Failed to decompress LZMA: {}", e));
            return;
        }
    };

    // Write audio to disk and convert to WAV (using SoX), deleting original file
    match fs::write("/home/{}/output/_temp.raw", audio.as_slice()) {
        Ok(_) => (),
        Err(e) => {
            logger.error(format!("Failed to write to file: {}", e));
            return;
        }
    };

    // sox -r 44100 -e signed -b 24 -c 2 temp.raw temp.wav
    Command::new("sox")
        .args(&[
            "-r",
            "44100",
            "-e",
            "signed",
            "-b",
            "24",
            "-c",
            "2",
            &format!("/home/{}/output/_temp.raw", username),
            &format!("/home/{}/output/_temp.wav", username),
        ])
        .spawn()
        .expect("Failed to run SoX");

    logger.info("Success!");
}
