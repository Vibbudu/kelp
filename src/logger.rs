use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{Event, Level, Metadata, Subscriber};
use tracing::span::{Attributes, Record};

pub struct FileLogger {
    log_dir: PathBuf,
    lock: Mutex<()>,
}

impl FileLogger {
    pub fn new() -> Self {
        let log_dir = crate::utilities::get_app_data_dir().join("logs");
        let _ = fs::create_dir_all(&log_dir);

        Self {
            log_dir,
            lock: Mutex::new(()),
        }
    }

    fn write_to_file(&self, filename: &str, content: &str) {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let filepath = self.log_dir.join(filename);

        // Auto-rotation: if file size > 5MB, rotate it
        if filepath.exists() {
            if let Ok(meta) = fs::metadata(&filepath) {
                if meta.len() > 5 * 1024 * 1024 {
                    let backup = self.log_dir.join(format!("{}.bak", filename));
                    let _ = fs::rename(&filepath, &backup);
                }
            }
        }

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&filepath)
        {
            let _ = writeln!(file, "{}", content.trim_end());
        }
    }
}

impl Subscriber for FileLogger {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &Event<'_>) {
        let metadata = event.metadata();
        let level = metadata.level();
        let target = metadata.target();
        
        let mut msg_visitor = StringVisitor::new();
        event.record(&mut msg_visitor);
        let message = msg_visitor.result;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let time_str = format_timestamp(now);

        let current_thread = thread::current();
        let thread_name = current_thread.name().unwrap_or("unnamed");

        let formatted = format!(
            "[{}] [{}] [{}] [{}] {}",
            time_str,
            thread_name,
            target,
            level.to_string(),
            message
        );

        // 1. Route to errors.log if ERROR or WARN
        if *level == Level::ERROR || *level == Level::WARN {
            self.write_to_file("errors.log", &formatted);
        }

        // 2. Route by target/module
        if target.contains("indexer") {
            self.write_to_file("indexer.log", &formatted);
        } else if target.contains("search") || target.contains("ranking") {
            self.write_to_file("search.log", &formatted);
        } else if target.contains("startup") || message.contains("Starting") || message.contains("initializing") {
            self.write_to_file("startup.log", &formatted);
        }

        // 3. Write all entries to launcher.log as master log
        self.write_to_file("launcher.log", &formatted);
    }

    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

struct StringVisitor {
    result: String,
}

impl StringVisitor {
    fn new() -> Self {
        Self { result: String::new() }
    }
}

impl tracing::field::Visit for StringVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.result = format!("{:?}", value);
            if self.result.starts_with('"') && self.result.ends_with('"') {
                self.result = self.result[1..self.result.len() - 1].to_string();
            }
        }
    }
}

fn format_timestamp(secs: u64) -> String {
    let days = secs / 86400;
    let seconds_in_day = secs % 86400;
    let hour = seconds_in_day / 3600;
    let minute = (seconds_in_day % 3600) / 60;
    let second = seconds_in_day % 60;

    let mut year = 1970;
    let mut day_count = days;
    loop {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if is_leap { 366 } else { 365 };
        if day_count >= days_in_year {
            day_count -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }
    
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, 1, day_count + 1, hour, minute, second)
}

/// Global Panic Handler Setup
pub fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|info| {
        let log_dir = crate::utilities::get_app_data_dir().join("logs");
        let _ = fs::create_dir_all(&log_dir);
        let filepath = log_dir.join("panic.log");

        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "Unknown panic payload"
        };

        let location = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_else(|| "unknown location".to_string());
        let backtrace = format!("{:?}", std::backtrace::Backtrace::force_capture());

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let time_str = format_timestamp(now);

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&filepath)
        {
            let _ = writeln!(
                file,
                "==================================================\n[{}] PANIC OCCURRED\nLocation: {}\nPayload: {}\nBacktrace:\n{}\n==================================================",
                time_str, location, payload, backtrace
            );
        }
    }));
}
