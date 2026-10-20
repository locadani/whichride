//! Outgoing HTTP clients: rustls with the ring provider (as sqlx), short timeouts.

use std::time::Duration;

use reqwest::redirect::Policy;

fn install_crypto_provider() {
    // Fails only if a provider is already installed, which is fine.
    let _ = rustls::crypto::ring::default_provider().install_default();
}

pub fn client(timeout: Duration) -> reqwest::Client {
    install_crypto_provider();
    reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .expect("a client with only a timeout always builds")
}

/// For short links: each redirect is inspected before it is followed (Task 11).
pub fn client_without_redirects(timeout: Duration) -> reqwest::Client {
    install_crypto_provider();
    reqwest::Client::builder()
        .timeout(timeout)
        .redirect(Policy::none())
        .build()
        .expect("a client with a timeout and no redirects always builds")
}
