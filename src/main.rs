#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate simple_logger;

mod gluster_brick;

use std::str::FromStr;

use clap::{Arg, App};
use log::LogLevel;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Args {
    influx_host: String,
    influx_port: u16,
    influx_username: String,
    influx_password: String,
}

fn main() {
    let matches = App::new("admin-gluster")
        .version(crate_version!())
        .author(crate_authors!())
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .help("Setup admin-gluster with a custom config file")
            .takes_value(true)
            .default_value("/etc/default/admin_gluster.yaml"))
        .arg(Arg::with_name("loglevel")
            .help("Sets the level to write the logs at")
            .long("loglevel")
            .takes_value(true)
            .default_value("info")
            .possible_values(&["off", "error", "warn", "info", "debug", "trace"])
            .required(false))
        .get_matches();

    // This should be safe since clap already validates that a valid value is input here
    let loglevel = LogLevel::from_str(matches.value_of("loglevel").unwrap()).unwrap();

    info!("Starting collection");
    simple_logger::init_with_level(loglevel).unwrap();
    info!("Logging with: {:?}", loglevel);

    gluster_brick::initialize_brick_scanner(&client);
    loop {
        std::thread::sleep(std::time::Duration::new(10, 0));
    }
}
