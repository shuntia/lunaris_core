use std::env;
use std::fmt;
use std::io::IsTerminal as _;
use std::sync::OnceLock;

static ANSI_ENABLED: OnceLock<bool> = OnceLock::new();

fn should_enable_ansi() -> bool {
    // App-specific override first
    match env::var("LUNARIS_COLOR").ok().as_deref() {
        Some("always") => return true,
        Some("never") => return false,
        _ => {}
    }
    // Honor common env vars first
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if matches!(env::var("CLICOLOR_FORCE").ok().as_deref(), Some("1")) {
        return true;
    }
    if matches!(env::var("CLICOLOR").ok().as_deref(), Some("0")) {
        return false;
    }
    // Fallback to TTY detection
    std::io::stdout().is_terminal()
}

pub fn init_log_global() {
    let ansi = should_enable_ansi();
    let _ = ANSI_ENABLED.set(ansi);
    // Keep `colored` output consistent with tracing's ANSI decision
    #[allow(deprecated)]
    {
        // colored 2.x uses this global override; it's a no-op if the crate version changes.
        colored::control::set_override(ansi);
    }

    use tracing_subscriber::{EnvFilter, filter::Directive, fmt::time::UtcTime};
    let formatter = LunarisFormatter {
        ansi,
        timer: UtcTime::rfc_3339(),
    };

    let mut filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    for directive in ["wgpu_core=warn", "wgpu_hal=warn", "naga=warn"] {
        if let Ok(dir) = directive.parse::<Directive>() {
            filter = filter.add_directive(dir);
        }
    }

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .event_format(formatter)
        .init();
}

pub fn ansi_enabled() -> bool {
    *ANSI_ENABLED.get().unwrap_or(&false)
}

use colored::Colorize;
use tracing::Event;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

#[derive(Clone)]
struct LunarisFormatter<T> {
    ansi: bool,
    timer: T,
}

impl<S, N, T> FormatEvent<S, N> for LunarisFormatter<T>
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    T: FormatTime + Send + Sync,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        // Timestamp (written directly)
        self.timer.format_time(&mut writer)?;
        write!(writer, " ")?;

        let meta = event.metadata();
        let level = meta.level();
        let target = meta.target();
        let file = meta.file().unwrap_or("?");
        let line = meta
            .line()
            .map(|l| l.to_string())
            .unwrap_or_else(|| "?".into());

        // Extract message and other fields
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        let message = visitor.take_message();
        let kv = visitor.format_kv();

        // Color helpers
        let (lvl_colored, width) = match *level {
            tracing::Level::ERROR => ("ERROR".red().bold().to_string(), 5),
            tracing::Level::WARN => ("WARN".yellow().bold().to_string(), 4),
            tracing::Level::INFO => ("INFO".green().bold().to_string(), 4),
            tracing::Level::DEBUG => ("DEBUG".blue().bold().to_string(), 5),
            tracing::Level::TRACE => ("TRACE".magenta().bold().to_string(), 5),
        };
        let lvl_padded = if self.ansi {
            format!("{lvl_colored: <width$}")
        } else {
            format!("{:<width$}", level, width = width)
        };

        let src = if self.ansi {
            format!("{}:{}", file, line).dimmed().to_string()
        } else {
            format!("{}:{}", file, line)
        };

        let target_s = if self.ansi {
            target.dimmed().to_string()
        } else {
            target.to_string()
        };

        // Base line
        write!(writer, "{} {}: {} ", lvl_padded, target_s, src)?;

        if let Some(msg) = message {
            write!(writer, "{}", msg)?;
            if !kv.is_empty() {
                write!(writer, " {}", kv)?;
            }
        } else if !kv.is_empty() {
            write!(writer, "{}", kv)?;
        }

        // Span context (from root)
        if let Some(curr) = ctx.lookup_current() {
            let scope = curr.scope();
            let spans: Vec<_> = scope.from_root().collect();
            if !spans.is_empty() {
                write!(writer, " ")?;
                if self.ansi {
                    write!(writer, "{}", "[".dimmed())?;
                } else {
                    write!(writer, "[")?;
                }
                for (i, span) in spans.iter().enumerate() {
                    if i > 0 {
                        if self.ansi {
                            write!(writer, "{}", ", ".dimmed())?;
                        } else {
                            write!(writer, ", ")?;
                        }
                    }
                    let name = span.name();
                    if self.ansi {
                        write!(writer, "{}", name.cyan())?;
                    } else {
                        write!(writer, "{}", name)?;
                    }
                }
                if self.ansi {
                    write!(writer, "{}", "]".dimmed())?;
                } else {
                    write!(writer, "]")?;
                }
            }
        }

        writeln!(writer)
    }
}

#[derive(Default)]
struct FieldVisitor {
    message: Option<String>,
    fields: Vec<String>,
}

impl FieldVisitor {
    fn take_message(&mut self) -> Option<String> {
        self.message.take()
    }
    fn format_kv(&self) -> String {
        if self.fields.is_empty() {
            String::new()
        } else {
            self.fields.join(" ")
        }
    }
}

impl tracing::field::Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        } else {
            self.fields.push(format!("{}={:?}", field.name(), value));
        }
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields.push(format!("{}=\"{}\"", field.name(), value));
        }
    }
}
