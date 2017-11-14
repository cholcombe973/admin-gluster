# admin-gluster
Exports Gluster performance metrics to Influxdb


### Configure
The default configuration file is located at:`/etc/default/admin_gluster.yaml`
and contains the following fields:
```
influx_url: "http://localhost:8086"
influx_database: "glusterfs"
influx_username: "user"
influx_password: "pass"
```
