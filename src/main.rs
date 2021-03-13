use jete::editor;
use jete::terminal::terminal_display;
use std::{env};
use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Root};

fn main() {
    configure_logging();

    let mut args = env::args_os();
    args.next().unwrap(); // safe: just the process name

    let file = args.next();

    let (display, inputs) = terminal_display();

    editor::run(file, display, inputs);

    
}


fn configure_logging() {
    let log_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S%.3f)} {l} {t} [{T}] - {m}{n}")))
        .build("jete.log")
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(log_file)))
        .build(Root::builder().appender("file").build(LevelFilter::Debug))
        .unwrap();
    
    let _ = log4rs::init_config(config).unwrap();

    log::debug!("logging initialized");

}