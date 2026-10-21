use std::{env, fs, path::PathBuf};

fn main() {
    const ID: &str = "KYUUTUBE_GOOGLE_CLIENT_ID";
    const SECRET: &str = "KYUUTUBE_GOOGLE_CLIENT_SECRET";
    println!("cargo:rerun-if-env-changed={ID}");
    println!("cargo:rerun-if-env-changed={SECRET}");
    let read = |name| env::var(name).ok().filter(|value| !value.trim().is_empty());
    let registration = match (read(ID), read(SECRET)) {
        (None, None) => "None".to_string(),
        (Some(id), Some(secret)) => {
            assert!(
                id.ends_with(".apps.googleusercontent.com")
                    && id.len() <= 256
                    && !id.chars().any(char::is_whitespace)
                    && secret.len() <= 256,
                "Invalid publisher Google Desktop OAuth registration"
            );
            // Installed-app registration is distributed in the binary. It cannot
            // be a confidential server secret. User tokens are never built in.
            format!("Some(kyuutube_youtube_account::ClientConfig {{ client_id: {id:?}.into(), client_secret: {secret:?}.into() }})")
        }
        _ => panic!(
            "Publisher OAuth requires both build settings; no user setup fallback is provided"
        ),
    };
    let file = PathBuf::from(env::var_os("OUT_DIR").expect("Missing Cargo output directory"))
        .join("publisher_oauth.rs");
    fs::write(file, format!("fn publisher_client() -> Option<kyuutube_youtube_account::ClientConfig> {{ {registration} }}\n"))
        .expect("Could not prepare publisher OAuth registration");
    tauri_build::build();
}
