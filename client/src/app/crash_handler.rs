use std::panic;

use native_dialog::MessageDialogBuilder;

pub fn init() {
    panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();

        let msg = match info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "An unknown error occurred.",
            },
        };

        let location = info
            .location()
            .map(|l| format!("File: {}, Line: {}", l.file(), l.line()))
            .unwrap_or_else(|| "Unknown location".to_string());

        let report = format!(
            "Kingdoms Game Engine has encountered a critical error. Do you want to print the crash report?\n\n\
            Error: {}\n\
            Location: {}\n\n\
            Stacktrace:\n{}",
            msg, location, backtrace
        );

        pollster::block_on(async {
            let yes = MessageDialogBuilder::default()
                .set_level(native_dialog::MessageLevel::Error)
                .set_title("Engine Crash")
                .set_text(&report)
                .confirm()
                .spawn()
                .await
                .unwrap();

            if yes {
                std::fs::write("crash_report.txt", report).ok();
            }
        });
    }));
}
