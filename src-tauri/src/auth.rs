use crate::error::{message, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::{timeout, Duration},
};
use url::Url;

const SERVICE: &str = "io.github.fabiouh.resonance";

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Credentials {
    pub client_secret: String,
    pub refresh_token: String,
}

pub fn load() -> Result<Credentials> {
    let entry = keyring::Entry::new(SERVICE, "google")
        .map_err(|_| message("Windows Credential Manager is unavailable."))?;
    match entry.get_password() {
        Ok(value) => serde_json::from_str(&value).map_err(Into::into),
        Err(keyring::Error::NoEntry) => Ok(Credentials::default()),
        Err(_) => Err(message(
            "Couldn't read Google credentials from Windows Credential Manager.",
        )),
    }
}

pub fn save(credentials: &Credentials) -> Result<()> {
    keyring::Entry::new(SERVICE, "google")
        .and_then(|entry| {
            entry.set_password(
                &serde_json::to_string(credentials).expect("serializable credentials"),
            )
        })
        .map_err(|_| message("Couldn't save Google credentials in Windows Credential Manager."))
}

pub fn clear() -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, "google")
        .map_err(|_| message("Windows Credential Manager is unavailable."))?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(message(
            "Couldn't remove Google credentials from Windows Credential Manager.",
        )),
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

pub fn http() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Resonance/0.1")
        .build()?)
}

pub async fn access_token(client_id: &str) -> Result<String> {
    let credentials = load()?;
    if credentials.refresh_token.is_empty() {
        return Err(message(
            "Sign in to Google in Settings to sync your playlists.",
        ));
    }
    let response = http()?
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id),
            ("client_secret", &credentials.client_secret),
            ("refresh_token", &credentials.refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(message(
            "Google session expired or the OAuth configuration changed. Sign in again in Settings.",
        ));
    }
    Ok(response.json::<TokenResponse>().await?.access_token)
}

fn callback_code(target: &str, state: &str) -> Result<String> {
    let url = Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| message("Invalid sign-in response."))?;
    if url.path() != "/callback"
        || !url
            .query_pairs()
            .any(|(key, value)| key == "state" && value == state)
    {
        return Err(message(
            "Sign-in response could not be verified. Try signing in again.",
        ));
    }
    url.query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or_else(|| message("Google sign-in was declined. You can try again in Settings."))
}

pub async fn sign_in(client_id: &str, secret: &str) -> Result<Credentials> {
    if !client_id.ends_with(".apps.googleusercontent.com") {
        return Err(message(
            "Enter a Google Desktop OAuth client ID in Settings first.",
        ));
    }
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let redirect = format!(
        "http://127.0.0.1:{}/callback",
        listener.local_addr()?.port()
    );
    let state = uuid::Uuid::new_v4().to_string();
    let verifier = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth").expect("constant URL");
    url.query_pairs_mut().extend_pairs([
        ("client_id", client_id),
        ("redirect_uri", &redirect),
        ("response_type", "code"),
        ("scope", "https://www.googleapis.com/auth/youtube.force-ssl"),
        ("state", &state),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
        ("access_type", "offline"),
        ("prompt", "consent"),
    ]);
    open::that(url.as_str())
        .map_err(|_| message("Couldn't open your browser for Google sign-in."))?;
    let code = timeout(Duration::from_secs(180), async {
        loop {
            let (mut socket, _) = listener.accept().await?;
            let mut request = vec![0; 8192];
            let read = match timeout(Duration::from_secs(5), socket.read(&mut request)).await {
                Ok(Ok(read)) => read,
                _ => continue,
            };
            let request = String::from_utf8_lossy(&request[..read]);
            let target = request.lines().next().and_then(|line| line.strip_prefix("GET ")).and_then(|line| line.split(' ').next()).unwrap_or_default();
            let result = callback_code(target, &state);
            let body = if result.is_ok() { "Sign-in received. Return to Resonance to finish." } else { "Sign-in could not be verified. Return to Resonance and try again." };
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            let _ = socket.write_all(response.as_bytes()).await;
            if target.starts_with("/callback?") { return result; }
        }
    }).await.map_err(|_| message("Google sign-in timed out. Try again in Settings."))??;
    let response = http()?
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id),
            ("client_secret", secret),
            ("code", &code),
            ("code_verifier", &verifier),
            ("redirect_uri", &redirect),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(message("Google couldn't complete sign-in. Check your Desktop OAuth credentials and consent-screen test users."));
    }
    let token = response.json::<TokenResponse>().await?;
    let refresh_token = token.refresh_token.ok_or_else(|| message("Google didn't grant offline access. Revoke Resonance access in your Google account and sign in again."))?;
    Ok(Credentials {
        client_secret: secret.into(),
        refresh_token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callback_requires_matching_state_and_path() {
        assert!(callback_code("/callback?code=code&state=wrong", "expected").is_err());
        assert!(callback_code("/elsewhere?code=code&state=expected", "expected").is_err());
        assert!(callback_code("/callback?error=access_denied&state=expected", "expected").is_err());
        assert_eq!(
            callback_code("/callback?code=a%2Bb&state=expected", "expected").unwrap(),
            "a+b"
        );
    }
}
