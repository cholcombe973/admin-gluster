extern crate gluster;
extern crate influent;
extern crate time;
use self::gluster::volume_list;

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::Duration;

use self::influent::client::{Client, Credentials};
use self::influent::create_client;

pub fn initialize_brick_scanner(args: &Args) {
    thread::spawn(move || {
        debug!("Monitoring Gluster Bricks");
        // Wait for 5 seconds and then proceed.
        let _ = timer(Duration::from_secs(5));

        let vols = match gluster::volume_list() {
            Some(vols) => vols,
            None => {
                error!("Unable to list gluster volumes.  Failed with error");
                return;
            }
        };

        // Grab the stats for each volume
        let stats_files = match fs::read_dir("/var/lib/glusterd/stats") {
            Ok(files) => files,
            Err(e) => {
                error!("Reading /var/lib/glusterd/stats failed with error: {:?}", e);
                return;
            }
        };

        let hostname = {
            let mut buffer: String = String::new();
            let bytes_read = File::open("/etc/hostname")
                .unwrap()
                .read_to_string(&mut buffer)
                .unwrap();
            buffer
        };
        let do_influx = args.influx.is_some() && args.outputs.contains(&"influx".to_string());
        let mut user = String::new();
        let mut password = String::new();
        let credentials: Credentials;
        let host: String;
        let mut hosts: Vec<&str> = vec![];

        let influx = args.influx;
        let client = if do_influx {
            let influx = influx.unwrap();
            user = influx.user.clone();
            password = influx.password.clone();
            credentials = Credentials {
                username: &user[..],
                password: &password[..],
                database: "ceph",
            };
            host = format!("http://{}:{}", influx.host, influx.port);
            hosts = vec![&host[..]];
            create_client(credentials, hosts)
        } else {
            credentials = Credentials {
                username: &user[..],
                password: &password[..],
                database: "",
            };
            create_client(credentials, hosts)
        };
        stats_files
            //Only operate on valid directory entries
            .filter(|entry| entry.is_ok())
            .map(|entry| entry.unwrap())
            // If the entry matches a volume name
            .filter(|entry| vols.contains(&entry.file_name().to_string_lossy().into_owned()))
            // Record the stats in influx
            .map(|entry| {
                let fops = split_and_parse_fops_json(&entry.path()).unwrap();
                influx::record_measurement(&fops.0, influx_client, &hostname, "brick_name");
                influx::record_measurement(&fops.1, influx_client, &hostname, "brick_name");
            });
    });
}

// Return a tuple of (aggr fops, inter fops)
fn split_and_parse_fops_json
    (path: &Path)
     -> Result<(HashMap<String, f64>, HashMap<String, f64>), ::std::io::Error> {
    let filename = path.file_name().unwrap().to_string_lossy();
    let mut buffer: String = String::new();
    let bytes_read = File::open(path)?
        .read_to_string(&mut buffer)?;
    let parts: Vec<&str> = buffer.split("}\n{").collect();
    let aggr_fops = gluster::fop::read_aggr_fop(parts[0], &filename).unwrap();
    let inter_fops = gluster::fop::read_inter_fop(parts[1], &filename).unwrap();

    Ok((aggr_fops, inter_fops))
}

fn timer(d: Duration) -> Receiver<()> {
    let (tx, rx) = channel();
    thread::spawn(move || loop {
        thread::sleep(d);
        if tx.send(()).is_err() {
            break;
        }
    });
    rx
}

mod influx {
    extern crate influent;

    use std::collections::HashMap;
    use super::time;

    use self::influent::measurement::{Measurement, Value};
    use self::influent::client::{Precision, Client};

    pub fn record_measurement(brick_fops: &HashMap<String, f64>,
                              client: &Client,
                              hostname: &str,
                              file_name: &str) {
        let mut measurement = Measurement::new("gluster_brick");
        measurement.add_tag("storage_type", "gluster");
        measurement.add_tag("volume_name", "");
        measurement.add_tag("type", "brick");
        measurement.set_timestamp(time::now().to_timespec().sec as i64);
        measurement.add_tag("hostname", hostname);

        // Get this from the filename
        //measurement.add_tag("brick_name", brick_name);
        //measurement.add_tag("volume_name", volume_name);

        for (name, value) in brick_fops {
            measurement.add_field(name, Value::Integer(*value as i64));
        }

        let _ = client.write_one(measurement, Some(Precision::Seconds));
    }
}
