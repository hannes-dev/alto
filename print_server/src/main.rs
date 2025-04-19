use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};

use escpos::driver::*;
use escpos::errors::PrinterError;
use escpos::printer::Printer;
use escpos::printer_options::PrinterOptions;
use escpos::utils::*;

use types::FaxMessage;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/fax", post(fax_message));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn fax_message(Json(fax): Json<FaxMessage>) -> Result<impl IntoResponse, Error> {
    dbg!(&fax);
    print_message(fax)?;
    Ok(StatusCode::OK)
}

fn print_message(fax: FaxMessage) -> Result<(), PrinterError> {
    let driver = NativeUsbDriver::open(0x04b8, 0x0e28).unwrap();
    Printer::new(driver, Protocol::default(), Some(PrinterOptions::default()))
        .init()?
        .debug_mode(Some(DebugMode::Dec))
        .justify(JustifyMode::LEFT)?
        .smoothing(true)?
        .writeln(&format!("{}", fax.time.format("%F %R")))?
        .feed()?
        .writeln(&fax.message)?
        .feed()?
        .writeln(&format!("{} - {}", fax.from, fax.ip))?
        .print_cut()?;

    Ok(())
}

enum Error {
    PrinterError(PrinterError),
    // Other(String),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let msg = match self {
            Error::PrinterError(e) => format!("Printer Error: {}", e),
            // Error::Other(e) => e,
        };

        (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
    }
}

impl From<PrinterError> for Error {
    fn from(e: PrinterError) -> Self {
        Error::PrinterError(e)
    }
}
