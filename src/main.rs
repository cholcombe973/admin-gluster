#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate simple_logger;

mod gluster_brick;

use std::fs::File;
use std::str::FromStr;

use clap::{Arg, App};
use log::LogLevel;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Args {
    pub influx_host: Option<String>,
    pub influx_port: Option<u16>,
    pub influx_username: Option<String>,
    pub influx_password: Option<String>,
}

fn main() {
    let matches = App::new("admin-gluster")
        .version(crate_version!())
        .author(crate_authors!())
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .help("Setup admin-gluster with a custom config file")
                .takes_value(true)
                .default_value("/etc/default/admin_gluster.yaml"),
        )
        .arg(
            Arg::with_name("scan_interval")
                .default_value("10")
                .short("s")
                .long("scaninterval")
                .help("Scan gluster stats every x seconds")
                .required(false)
                .takes_value(true)
                .validator(|val| match u64::from_str(&val) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e.to_string()),
                }),
        )
        .arg(
            Arg::with_name("loglevel")
                .help("Sets the level to write the logs at")
                .long("loglevel")
                .takes_value(true)
                .default_value("info")
                .possible_values(&["off", "error", "warn", "info", "debug", "trace"])
                .required(false),
        )
        .get_matches();

    // This should be safe since clap already validates that a valid value is input here
    let loglevel = LogLevel::from_str(matches.value_of("loglevel").unwrap()).unwrap();
    // This is safe because we're using a validator function above
    let interval = u64::from_str(matches.value_of("scan_interval").unwrap()).unwrap();

    info!("Starting collection");
    simple_logger::init_with_level(loglevel).unwrap();
    info!("Logging with: {:?}", loglevel);

    let config_file = match File::open(matches.value_of("config").unwrap()) {
        Ok(f) => f,
        Err(e) => {
            error!(
                "Failed to open {}, Error: {}",
                matches.value_of("config").unwrap(),
                e
            );
            return;
        }
    };

    let config: Args = match serde_yaml::from_reader(config_file) {
        Ok(f) => f,
        Err(e) => {
            error!(
                "Failed to parse {}.  Error: {}",
                matches.value_of("config").unwrap(),
                e
            );
            return;
        }
    };

    gluster_brick::initialize_brick_scanner(config, interval);
    loop {
        std::thread::sleep(std::time::Duration::new(10, 0));
    }
}
