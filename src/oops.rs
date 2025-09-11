use lunaris_api::util::error::LunarisError;
use native_dialog::MessageDialogBuilder;
use notify_rust::Notification;

pub struct Oops {
    reason: LunarisError,
}

impl Oops {
    pub fn notify(&self) {
        let _ = Notification::new()
            .summary("Lunaris errored out")
            .body(&self.reason.to_string())
            .show();
    }
    pub fn popup(&self) {
        let _ = MessageDialogBuilder::default()
            .set_title("Lunaris Error")
            .set_text(self.reason.to_string())
            .set_level(native_dialog::MessageLevel::Error)
            .alert()
            .show();
    }
}
