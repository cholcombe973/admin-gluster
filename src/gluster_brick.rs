extern crate gluster;
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


pub fn initialize_brick_scanner() {
    thread::spawn(move || {
        debug!("Monitoring Gluster Bricks");
        // Wait for 5 seconds and then proceed.
        let _ = timer(Duration::from_secs(5));

        let vols = gluster::volume_list().unwrap();
        // Grab the stats for each volume
        let stats_files = match fs::read_dir("/var/lib/glusterd/stats") {
            Ok(files) => files,
            Err(e) => {
                error!("Reading /var/lib/glusterd/stats failed with error: {:?}", e);
                return;
            }
        };

        for file in stats_files {
            for vol in vols {
                // Skipping invalid direntries
                if let Ok(f) = file {
                    if f.file_name().to_string_lossy() == format!("glusterfs_{}.dump", vol) {
                        // Read this vol fops file
                        // Split the aggr from the inter fops.  This file technically isn't valid
                        // json because it concats 2 objects together

                        if let Ok(f_type) = f.file_type() {
                            // Check if this is a file just in case
                            if f_type.is_file() {

                                // influx::record_measurement(brick_fops: &HashMap<String, f64>,
                                //                           client: &Client,
                                //                           hostname: &str,
                                //                           drive_name: &str,
                                //                           osd_num: &str)
                            } else {
                                error!("The glusterfs_{}.dump file is not a file!", vol);
                            }
                        }
                    } else {
                        // Skipping entries that don't look like glusterfs_{vol_name}.dump
                        trace!("Skipping fop stat file: {}",
                               f.file_name().to_string_lossy());
                    }
                }
            }
        }
    });
}

// Return a tuple of (aggr fops, inter fops)
fn split_and_parse_fops_json
    (path: &Path)
     -> Result<(HashMap<String, f64>, HashMap<String, f64>), ::std::io::Error> {
    let mut buffer: String = String::new();
    let bytes_read = File::open(path)?
        .read_to_string(&mut buffer)?;
    let parts = buffer.split("}\n{").collect();
    gluster::fop::read_aggr_fop(json_data: &str, filename: &str)

    Ok(())
}

fn timer(d: Duration) -> Receiver<()> {
    let (tx, rx) = channel();
    thread::spawn(move || {
        loop {
            thread::sleep(d);
            if tx.send(()).is_err() {
                break;
            }
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
                              brick_name: &str,
                              volume_name: &str) {
        let mut measurement = Measurement::new("gluster_brick");
        measurement.add_tag("type", "brick");
        measurement.set_timestamp(time::now().to_timespec().sec as i64);
        measurement.add_tag("hostname", hostname);
        measurement.add_tag("brick_name", brick_name);
        measurement.add_tag("volume_name", volume_name);

        measurement.add_field("load_average", Value::Integer(osd_m.load_average as i64));

        // Send Influxdb the diff between last and now.
        measurement.add_field("subop", Value::Integer(osd_m.subop as i64));
        measurement.add_field("subop_in_bytes",
                              Value::Integer(osd_m.subop_in_bytes as i64));

        measurement.add_field("op_r", Value::Integer(osd_m.op_r as i64));
        measurement.add_field("op_r_bytes", Value::Integer(osd_m.op_r_bytes as i64));
        measurement.add_field("op_w", Value::Integer(osd_m.op_w as i64));
        measurement.add_field("op_w_bytes", Value::Integer(osd_m.op_w_bytes as i64));

        let _ = client.write_one(measurement, Some(Precision::Seconds));
    }
}
