use std::sync::{RwLock, OnceLock};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
    NoLog = 99,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "debug" | "trace" => LogLevel::Debug,
            "warn" | "warning" => LogLevel::Warn,
            "error" | "fatal" => LogLevel::Error,
            "nolog" | "off" => LogLevel::NoLog,
            _ => LogLevel::Info,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::NoLog => "NOLOG",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoggerOptions {
    pub level: LogLevel,
    pub enabled: bool,
    pub log_to_file: bool,
    pub log_dir: String,
    pub log_filename: String,
}

impl Default for LoggerOptions {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            enabled: true,
            log_to_file: false,
            log_dir: "./logs".to_string(),
            log_filename: String::new(),
        }
    }
}

static GLOBAL_OPTIONS: OnceLock<RwLock<LoggerOptions>> = OnceLock::new();

pub fn global_options() -> &'static RwLock<LoggerOptions> {
    GLOBAL_OPTIONS.get_or_init(|| RwLock::new(LoggerOptions::default()))
}

pub fn setup_logger(
    level: Option<String>,
    log_to_file: Option<bool>,
    log_dir: Option<String>,
    log_filename: Option<String>,
) {
    let mut opts = global_options().write().unwrap();
    if let Some(l) = level {
        opts.level = LogLevel::from_str(&l);
    }
    if let Some(ltf) = log_to_file {
        opts.log_to_file = ltf;
    }
    if let Some(ld) = log_dir {
        opts.log_dir = ld;
    }
    if let Some(lf) = log_filename {
        opts.log_filename = lf;
    }
}

#[derive(Clone, Debug)]
pub struct Logger {
    module_name: String,
}

impl Logger {
    pub fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
        }
    }

    pub fn child(&self, module_name: &str) -> Self {
        Self {
            module_name: format!("{}/{}", self.module_name, module_name),
        }
    }

    fn should_log(&self, msg_level: LogLevel) -> bool {
        let opts = global_options().read().unwrap();
        opts.enabled && opts.level != LogLevel::NoLog && msg_level >= opts.level
    }

    fn dispatch(&self, level: LogLevel, msg: &str) {
        if !self.should_log(level) {
            return;
        }

        let now = chrono::Local::now();
        let time_str = now.format("%Y-%m-%dT%H:%M:%S%.3f%z").to_string();

        let color_code = match level {
            LogLevel::Debug => "\x1b[36m", // Cyan
            LogLevel::Info => "\x1b[32m",  // Green
            LogLevel::Warn => "\x1b[33m",  // Yellow
            LogLevel::Error => "\x1b[31m", // Red
            _ => "\x1b[0m",
        };
        let reset_code = "\x1b[0m";

        // Match the legacy colored prefix format: prefixCol + message
        let legacy_colored = format!(
            "{}{} [{}] [{}]{} {}",
            color_code, time_str, self.module_name, level.to_str(), reset_code, msg
        );

        // Print to console
        match level {
            LogLevel::Error => eprintln!("{}", legacy_colored),
            LogLevel::Warn => eprintln!("{}", legacy_colored),
            _ => println!("{}", legacy_colored),
        }

        // Write to file
        let opts = global_options().read().unwrap();
        if opts.log_to_file {
            let date_str = now.format("%Y-%m-%d").to_string();
            let filename = if opts.log_filename.is_empty() {
                format!("{}.log", date_str)
            } else {
                format!("{}-{}.log", date_str, opts.log_filename)
            };

            let log_dir = Path::new(&opts.log_dir);
            if !log_dir.exists() {
                let _ = fs::create_dir_all(log_dir);
            }

            let file_path = log_dir.join(filename);
            
            // Strip ANSI escapes
            let clean_msg = strip_ansi_escapes(msg);
            let file_line = format!(
                "{} [{}] [{}] {}",
                time_str,
                self.module_name,
                level.to_str(),
                clean_msg
            );

            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(file_path)
            {
                let _ = writeln!(file, "{}", file_line);
            }
        }
    }

    pub fn debug(&self, msg: &str) { self.dispatch(LogLevel::Debug, msg); }
    pub fn info(&self, msg: &str) { self.dispatch(LogLevel::Info, msg); }
    pub fn warn(&self, msg: &str) { self.dispatch(LogLevel::Warn, msg); }
    pub fn error(&self, msg: &str) { self.dispatch(LogLevel::Error, msg); }
    pub fn fatal(&self, msg: &str) { self.dispatch(LogLevel::Error, msg); }
    pub fn trace(&self, msg: &str) { self.dispatch(LogLevel::Debug, msg); }
}

fn strip_ansi_escapes(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if let Some('[') = chars.peek() {
                chars.next(); // consume '['
                in_escape = true;
                continue;
            }
        }
        if in_escape {
            if c == 'm' {
                in_escape = false;
            }
            continue;
        }
        result.push(c);
    }
    result
}
