use jete::editor;
use log::LevelFilter;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::{
    policy::compound::{roll::delete::DeleteRoller, trigger::size::SizeTrigger},
    RollingFileAppender,
};
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use std::env;

fn main() {
    configure_logging();

    let mut args = env::args_os();
    args.next().unwrap(); // safe: just the process name

    let file = args.next();

    editor::run(file);
}

fn configure_logging() {
    let roll_policy = Box::new(CompoundPolicy::new(
        Box::new(SizeTrigger::new(5_000_000)),
        Box::new(DeleteRoller::new()),
    ));

    let log_file = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S%.3f)} {l} {t} [{T}:{I}] - {m}{n}",
        )))
        .build("jete.log", roll_policy)
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(log_file)))
        .build(Root::builder().appender("file").build(LevelFilter::Debug))
        .unwrap();

    let _ = log4rs::init_config(config).unwrap();

    let existing_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |pi| {
        let (file, line) = pi
            .location()
            .map(|l| (l.file(), l.line()))
            .unwrap_or(("unknown", 0));
        let msg = pi
            .payload()
            .downcast_ref::<&str>()
            .unwrap_or(&"(no message)");
        log::error!("panic occurred [{}:{}]: {}", file, line, msg);

        existing_hook(pi);
    }));

    log::debug!("logging initialized");
}
