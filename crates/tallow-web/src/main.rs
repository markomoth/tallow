//! Tallow in the browser: the same app, screens and log as the terminal
//! binary, drawn on a WebGL canvas by Ratzilla. Build it with `trunk` from
//! this directory (see `index.html`).

#[cfg(target_arch = "wasm32")]
mod web;

fn main() {
    #[cfg(target_arch = "wasm32")]
    web::start();
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("tallow-web runs in a browser: `trunk serve` in crates/tallow-web");
}
