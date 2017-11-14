extern crate gluster;
extern crate influent;
extern crate time;

use super::Args;

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::Duration;

use self::influent::client::Credentials;
use self::influent::create_client;

pub fn initialize_brick_scanner(args: Args, scan_interval: u64) {
    thread::spawn(move || {
        debug!("Monitoring Gluster Bricks");
        // Wait for x seconds and then proceed.
        let _ = timer(Duration::new(scan_interval, 0));

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
            match File::open("/etc/hostname").and_then(|mut f| f.read_to_string(&mut buffer)) {
                Ok(_) => buffer,
                Err(e) => {
                    error!("Eror reading /etc/hostname: {}", e);
                    return;
                }
            }
        };
        let username = args.influx_username.unwrap_or("".to_string());
        let password = args.influx_password.unwrap_or("".to_string());
        let influx_host = args.influx_url.unwrap_or(
            "http://localhost:8086".to_string(),
        );
        let db = args.influx_database.unwrap_or("glusterfs".to_string());

        for stat_file in stats_files {
            //Only operate on valid directory entries
            if !stat_file.is_ok() {
                warn!("Skipping error file: {:?}", stat_file);
                continue;
            }
            let e = stat_file.unwrap();
            let filename = e.path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .replace(".dump", "")
                .replace("glusterfsd_", "");
            // If the entry matches a volume name
            let fops = split_and_parse_fops_json(&e.path()).unwrap();
            influx::record_measurement(
                &fops.0,
                &username,
                &password,
                &influx_host,
                &db,
                &hostname,
                &filename,
            );
            influx::record_measurement(
                &fops.1,
                &username,
                &password,
                &influx_host,
                &db,
                &hostname,
                &filename,
            );

        }
    });
}

// Return a tuple of (aggr fops, inter fops)
fn split_and_parse_fops_json(
    path: &Path,
) -> Result<(HashMap<String, f64>, HashMap<String, f64>), ::std::io::Error> {
    let filename = path.file_name()
        .unwrap()
        .to_string_lossy()
        .replace(".dump", "")
        .replace("glusterfsd_", "");
    let mut buffer: String = String::new();
    File::open(path)?.read_to_string(&mut buffer)?;
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
    extern crate reqwest;

    use std::collections::HashMap;
    use super::time;

    use self::influent::client::Precision;
    use self::influent::serializer::Serializer;
    use self::influent::serializer::line::LineSerializer;
    use self::influent::measurement::{Measurement, Value};

    pub fn record_measurement(
        brick_fops: &HashMap<String, f64>,
        username: &str,
        password: &str,
        influx_url: &str,
        database: &str,
        hostname: &str,
        brick_name: &str,
    ) {
        let mut measurement = Measurement::new("gluster_brick");
        measurement.add_tag("storage_type", "gluster");
        measurement.add_tag("volume_name", "");
        measurement.add_tag("type", "brick");
        measurement.set_timestamp(time::now().to_timespec().sec as i64);
        measurement.add_tag("hostname", hostname);

        // Get this from the filename
        measurement.add_tag("brick_name", brick_name);

        // Add all fields collected from gluster
        for (name, value) in brick_fops {
            measurement.add_field(name, Value::Integer(*value as i64));
        }
        let serializer = LineSerializer::new();
        let serial_data = serializer.serialize(&measurement);

        let client = reqwest::Client::new();
        let res = client
            .post(&format!(
                "{influx_url}/write?db={database}&precision={precision}",
                influx_url = influx_url,
                database = database,
                precision = Precision::Seconds.to_string()
            ))
            .body(serial_data)
            .send();
        info!("write result: {:?}", res);

        //let _ = client.write_one(measurement, Some(Precision::Seconds));
    }
}
