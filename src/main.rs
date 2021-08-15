#[macro_use]
extern crate log;

use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use tokio::net::TcpListener;

use tokio_rustls::rustls::internal::pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::rustls::{NoClientAuth, ServerConfig};
use tokio_rustls::TlsAcceptor;

use axum::prelude::*;
use tower_http::set_header::SetResponseHeaderLayer;

use anyhow::{Error, Result};
use fern::colors::{Color, ColoredLevelConfig};
use hyper::server::conn::Http;
use log::LevelFilter;
use structopt::StructOpt;

mod routes;
mod utils;

use axum::http::header;
use engine::SearchEngine;
use hyper::http::header::SERVER;
use hyper::http::HeaderValue;
use tower::ServiceBuilder;
use tower_http::auth::RequireAuthorizationLayer;

#[derive(Debug, StructOpt)]
#[structopt(name = "lnx", about = "A ultra-fast, adaptable search engine.")]
struct Settings {
    /// The log level filter, any logs that are above this level wont
    /// be displayed.
    #[structopt(long, default_value = "info", env)]
    log_level: LevelFilter,

    /// An optional bool to use ASNI colours for log levels.
    /// You probably want to disable this if using file based logging.
    #[structopt(long, env)]
    pretty_logs: Option<bool>,

    /// The host to bind to (normally: '127.0.0.1' or '0.0.0.0'.)
    #[structopt(long, short, default_value = "127.0.0.1", env)]
    host: String,

    /// The port to bind the server to.
    #[structopt(long, short, default_value = "8000", env)]
    port: u16,

    /// If specified this will be used in the TLS config for HTTPS.
    #[structopt(long, env)]
    tls_key_file: Option<String>,

    /// If specified this will be used in the TLS config for HTTPS.
    #[structopt(long, env)]
    tls_cert_file: Option<String>,

    /// If specified this will require an authentication key on each request.
    ///
    /// Generally it's recommend to have this in a production environment.
    #[structopt(long, short = "auth", env, hide_env_values = true)]
    authentication_key: Option<String>,

    /// The amount of threads to use for the tokio runtime.
    ///
    /// If this is not set, the number of logical cores on the machine is used.
    #[structopt(long, short = "threads", env)]
    runtime_threads: Option<usize>,

    /// A optional file to send persistent logs.
    #[structopt(long, env)]
    log_file: Option<String>,
}

fn setup_logger(level: LevelFilter, log_file: &Option<String>, pretty: bool) -> Result<()> {
    let mut colours = ColoredLevelConfig::new();

    if pretty {
        colours = colours
            .info(Color::Green)
            .warn(Color::Yellow)
            .error(Color::BrightRed)
            .debug(Color::Magenta)
            .trace(Color::Cyan);
    }

    let mut builder = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{} | {} | {:<5} - {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                colours.color(record.level()),
                message,
            ))
        })
        .level(level)
        .chain(std::io::stdout());

    if let Some(file) = log_file {
        builder = builder.chain(fern::log_file(file)?);
    }

    builder.apply()?;

    Ok(())
}

fn main() {
    let settings = match setup() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error during server setup: {:?}", e);
            return;
        }
    };

    let threads = settings.runtime_threads.unwrap_or_else(|| num_cpus::get());
    info!("starting runtime with {} threads", threads);
    let maybe_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(threads)
        .enable_all()
        .build();

    let result = match maybe_runtime {
        Ok(runtime) => runtime.block_on(start(settings)),
        Err(e) => {
            error!("error during runtime creation: {:?}", e);
            return;
        }
    };

    if let Err(e) = result {
        error!("error during server runtime: {:?}", e);
    }
}

/// Parses the config and sets up logging
fn setup() -> Result<Settings> {
    let config: Settings = Settings::from_args();
    setup_logger(
        config.log_level,
        &config.log_file,
        config.pretty_logs.unwrap_or(true),
    )?;
    Ok(config)
}

/// Starts the server in an async context.
async fn start(settings: Settings) -> Result<()> {
    let tls = check_tls_files(&settings)?;

    let engine = Arc::new(SearchEngine::create("/lnx/meta").await?);

    let app = route("/indexes/:index_name/search", get(routes::search_index))
        .route("/indexes/:index_name", delete(routes::delete_index))
        .route("/indexes", post(routes::create_index))
        .route(
            "/indexes/:index_name/documents/:document_id",
            delete(routes::delete_document).get(routes::get_document),
        )
        .route(
            "/indexes/:index_name/documents",
            post(routes::add_document).delete(routes::delete_all_documents),
        )
        .layer(RequireAuthorizationLayer::custom(
            utils::AuthIfEnabled::bearer(
                settings
                    .authentication_key
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or_else(|| ""),
                settings.authentication_key.is_some(),
                "Missing token bearer authorization header.",
            ),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::SERVER,
            HeaderValue::from_static("lnx"),
        ));

    let addr = format!("{}:{}", &settings.host, settings.port);
    let handle = match tls {
        Some(tls) => tokio::spawn(async move {
            info!("starting https server @ {}", addr);

            let acceptor = TlsAcceptor::from(tls);
            let listener = TcpListener::bind(&addr).await?;

            loop {
                let (stream, _addr) = listener.accept().await?;
                let acceptor = acceptor.clone();

                let ap = app.clone();

                tokio::spawn(async move {
                    if let Ok(stream) = acceptor.accept(stream).await {
                        if let Err(e) = Http::new().serve_connection(stream, ap).await {
                            warn!("failed to serve connection: {:?}", e);
                        };
                    }
                });
            }
        }),
        None => tokio::spawn(async move {
            info!("starting http server @ {}", addr);
            axum::Server::bind(&addr.parse()?)
                .serve(app.into_make_service())
                .await?;

            Ok::<(), Error>(())
        }),
    };

    tokio::signal::ctrl_c().await?;
    info!("shutting down server...");

    handle.abort();
    Ok(())
}

/// Validates that both a key and cert file has been provided or none have
/// been provided.
fn check_tls_files(settings: &Settings) -> Result<Option<Arc<ServerConfig>>> {
    match (&settings.tls_key_file, &settings.tls_cert_file) {
        (Some(fp1), Some(fp2)) => Ok(Some(tls_server_config(fp1, fp2)?)),
        (None, None) => Ok(None),
        _ => {
            return Err(Error::msg(
                "missing a required TLS field, both key and cert must be provided.",
            ))
        }
    }
}

/// Parses and handles a given key and cert for TLS.
fn tls_server_config(key: &str, cert: &str) -> Result<Arc<ServerConfig>> {
    let mut config = ServerConfig::new(NoClientAuth::new());

    let mut key_reader = BufReader::new(File::open(key)?);
    let mut cert_reader = BufReader::new(File::open(cert)?);

    let key = pkcs8_private_keys(&mut key_reader)
        .map_err(|_| Error::msg("failed to extract private keys"))?
        .remove(0);

    let certs =
        certs(&mut cert_reader).map_err(|_| Error::msg("failed to extract certificates"))?;

    config.set_single_cert(certs, key)?;

    config.set_protocols(&[b"h2".to_vec(), b"http/1.1".to_vec()]);

    Ok(Arc::new(config))
}
