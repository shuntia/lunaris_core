use std::process::{abort, exit};

use lunaris_api::util::error::{LunarisError, Result};
use native_dialog::DialogBuilder;
use signal_hook::{
    consts::{SIGABRT, SIGINT},
    low_level::register,
};
use tracing::*;

pub fn register_hooks() -> Result {
    unsafe {
        register(SIGINT, || {
            let _ = DialogBuilder::message()
                .set_title("SIGINT")
                .set_text("Received SIGINT. Aborting program.")
                .set_level(native_dialog::MessageLevel::Error)
                .alert()
                .show();
            error!("SIGINT Received; Attempting to save...");
            error!("SIGINT not implemented.");
            abort();
        })
        .map_err(|e| LunarisError::KernelInitFailed {
            reason: format!("{e}"),
        })?;
        register(SIGABRT, || {
            error!("Aborting(SIGABRT)");
            exit(1)
        })
        .map_err(|e| LunarisError::KernelInitFailed {
            reason: format!("{e}"),
        })?;
    }
    Ok(())
}
