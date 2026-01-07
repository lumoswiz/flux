use std::path::PathBuf;

use alloy::{primitives::hex, signers::local::LocalSigner};
use eyre::{Result, eyre};
use rand::thread_rng;

/// Create a new keystore from a private key
pub fn add(output: Option<String>) -> Result<()> {
    // Determine output path
    let output_path = match output {
        Some(path) => PathBuf::from(path),
        None => {
            let flux_dir = dirs::home_dir()
                .ok_or_else(|| eyre!("Could not determine home directory"))?
                .join(".flux");
            std::fs::create_dir_all(&flux_dir)?;
            flux_dir.join("keystore.json")
        }
    };

    // Check if file already exists
    if output_path.exists() {
        return Err(eyre!(
            "Keystore already exists at {}. Remove it first or specify a different path with --output",
            output_path.display()
        ));
    }

    // Prompt for private key
    let private_key_input =
        rpassword::prompt_password("Enter private key (hex, with or without 0x prefix): ")?;
    let private_key_hex = private_key_input.trim().trim_start_matches("0x");

    let private_key_bytes: [u8; 32] = hex::decode(private_key_hex)
        .map_err(|e| eyre!("Invalid private key hex: {}", e))?
        .try_into()
        .map_err(|_| eyre!("Private key must be exactly 32 bytes"))?;

    // Prompt for password
    let password = rpassword::prompt_password("Enter password for keystore: ")?;
    let password_confirm = rpassword::prompt_password("Confirm password: ")?;

    if password != password_confirm {
        return Err(eyre!("Passwords do not match"));
    }

    if password.is_empty() {
        return Err(eyre!("Password cannot be empty"));
    }

    // Create parent directory if needed
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create keystore
    let mut rng = thread_rng();
    let (wallet, _) = LocalSigner::encrypt_keystore(
        output_path.parent().unwrap_or(&PathBuf::from(".")),
        &mut rng,
        private_key_bytes,
        &password,
        Some(output_path.file_name().unwrap().to_str().unwrap()),
    )
    .map_err(|e| eyre!("Failed to create keystore: {}", e))?;

    println!("Keystore created successfully!");
    println!("  Address: {}", wallet.address());
    println!("  Path: {}", output_path.display());

    Ok(())
}
