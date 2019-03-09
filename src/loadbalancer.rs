use std::collections::HashMap;
use std::time::{Instant, Duration};
use std::thread::sleep;
use prettytable::format;
use prettytable::Table;
use chrono::{DateTime, Local, SecondsFormat, Duration as ChronoDuration};
use rusoto_elb::{Elb, ElbClient, DescribeEndPointStateInput};
use rusoto_ec2::{Ec2, Ec2Client, DescribeInstancesRequest};
use rusoto_cloudwatch::{CloudWatch, CloudWatchClient, Dimension, GetMetricStatisticsInput};

#[derive(Debug)]
struct InstanceStatus {
    id:      std::string::String,
    health:  std::string::String,
    name:    std::string::String,
    version: std::string::String,
    address: std::string::String,
    asg:     std::string::String,
    uptime:  std::string::String,
}

pub fn elb_status(m: &clap::ArgMatches, r: rusoto_core::Region) {
    let elb_client = ElbClient::new(r.to_owned());
    let ec2_client = Ec2Client::new(r.to_owned());

    let elb_state = DescribeEndPointStateInput {
        load_balancer_name: m.value_of("name").unwrap().to_string(),
        ..Default::default()
    };

    let mut instance_data: Vec<InstanceStatus> = Vec::new();

    let health_results = elb_client.describe_instance_health(elb_state).sync().ok().unwrap();

    for x in health_results.instance_states.unwrap() {
        let ec2_instance = DescribeInstancesRequest {
            instance_ids: Some(vec![x.instance_id.clone().unwrap()]),
            ..Default::default()
        };
        let instance_results = ec2_client.describe_instances(ec2_instance).sync().ok();
        let instance_reservation = &instance_results.clone().unwrap().reservations.unwrap()[0];
        let instance_uptime = instance_reservation.clone().instances.unwrap().into_iter().nth(0).unwrap().launch_time.unwrap();
        let instance_address = instance_reservation.clone().instances.unwrap().into_iter().nth(0).unwrap().private_ip_address.unwrap();
        let instance_tags = instance_reservation.clone().instances.unwrap().into_iter().nth(0).unwrap().tags.unwrap();
        let mut instance_name: std::string::String = "".to_string();
        let mut instance_version: std::string::String = "".to_string();
        let mut instance_asg: std::string::String = "".to_string();
        for i in instance_tags {
            if i.key == Some("Name".to_string()) {
                instance_name = i.value.clone().unwrap();
            };
            if i.key == Some("version".to_string()) {
                instance_version = i.value.clone().unwrap();
            };
            if i.key == Some("aws:autoscaling:groupName".to_string()) {
                instance_asg = i.value.clone().unwrap();
            };
        };
        instance_data.push(
            InstanceStatus {
                name: instance_name,
                version: instance_version,
                id: x.instance_id.clone().unwrap(),
                health: x.state.clone().unwrap(),
                uptime: instance_uptime.to_string(),
                address: instance_address.to_string(),
                asg: instance_asg
            }
        )
    };

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(row!["Instance ID", "Health", "Name", "Version", "IP Address", "ASG", "Uptime"]);

    for (y, _z) in instance_data.iter().enumerate() {
        let dt_local = Local::now();
        let mut dt_f = timeago::Formatter::new();
        dt_f.num_items(3);
        let dt_inst = DateTime::parse_from_rfc3339(&instance_data[y as usize].uptime);
        let dt_dur = dt_local.signed_duration_since(dt_inst.unwrap());
        table.add_row(row![    
            instance_data[y as usize].id,
            instance_data[y as usize].health,
            instance_data[y as usize].name,
            instance_data[y as usize].version,
            instance_data[y as usize].address,
            instance_data[y as usize].asg,
            dt_f.convert(dt_dur.to_std().unwrap())
        ]);
    };
    table.printstd();
}

pub fn elb_stats(r: rusoto_core::Region, n: String, interval: i64) -> Vec<f64> {
    let cw_client = CloudWatchClient::new(r.to_owned());

    let mut stat_requests = Vec::new();

    let mut request_count = HashMap::new();
    request_count.insert("metricName".to_string(), "RequestCount".to_string());
    request_count.insert("statistic".to_string(), "Sum".to_string());
    stat_requests.push(request_count);

    let mut connection_errors = HashMap::new();
    connection_errors.insert("metricName".to_string(), "BackendConnectionErrors".to_string());
    connection_errors.insert("statistic".to_string(), "Sum".to_string());
    stat_requests.push(connection_errors);

    let mut is_errors = HashMap::new();
    is_errors.insert("metricName".to_string(), "HTTPCode_Backend_5XX".to_string());
    is_errors.insert("statistic".to_string(), "Sum".to_string());
    stat_requests.push(is_errors);

    let mut latency = HashMap::new();
    latency.insert("metricName".to_string(), "Latency".to_string());
    latency.insert("statistic".to_string(), "Average".to_string());
    stat_requests.push(latency);

    let dt_local = Local::now();
    let dt_mod_local = dt_local - ChronoDuration::minutes(interval);
    let mut stat_vector = Vec::new();
    stat_vector.push(interval as f64);

    for r in stat_requests.clone() {
        let elb_name = Dimension {
            name: "LoadBalancerName".to_string(),
            value: n.to_string(),
        };

        let stat_req = GetMetricStatisticsInput {
            start_time: dt_mod_local.to_rfc3339_opts(SecondsFormat::Secs, true),
            end_time: dt_local.to_rfc3339_opts(SecondsFormat::Secs, true),
            period: ChronoDuration::minutes(interval).num_seconds(),
            metric_name: r.get("metricName").unwrap().to_string(),
            namespace: "AWS/ELB".to_string(),
            statistics: Some(vec![r.get("statistic").unwrap().to_string()]),
            dimensions: Some(vec![elb_name]),
            ..Default::default()
        };

        if r.get("statistic").unwrap().to_string() == "Sum".to_string() {
            match cw_client.get_metric_statistics(stat_req.clone()).sync() {
                Ok(k) => stat_vector.push(k.datapoints.map(|dp| { if dp.is_empty() { return 0.0 } dp[0].sum.unwrap_or(0.0) as f64}).unwrap()),
                Err(error) => eprintln!("ERROR: {:?}", error),
            };
        };
        if r.get("statistic").unwrap().to_string() == "Average".to_string() {
            match cw_client.get_metric_statistics(stat_req.clone()).sync() {
                Ok(k) => stat_vector.push(k.datapoints.map(|dp| { if dp.is_empty() { return 0.0 } dp[0].average.unwrap_or(0.0) as f64}).unwrap()),
                Err(error) => eprintln!("ERROR: {:?}", error),
            };
        };
    };

    return stat_vector
}

pub fn elb_stats_cmd(r: rusoto_core::Region, m: &clap::ArgMatches) {
    let intervals = vec![1, 5, 15, 60];

    let mut instance_stats = Vec::new();

    for i in intervals {
        let interval_stats = elb_stats(r.clone(), m.value_of("name").unwrap().to_string(), i);
        instance_stats.push(interval_stats);
    };

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(row!["Interval", "Requests", "Requests/sec", "500 Errors", "500 Error %", "Connection Errors", "Avg. Latency (ms)"]);

    for stat in instance_stats {
        table.add_row(row![
            format!("{} min", stat[0]),
            stat[1],
            format!("{:.2}", stat[1] / (stat[0] * 60.0)),
            stat[3],
            format!("{:.3}", stat[3] / stat[1]),
            stat[2],
            format!("{:.3}", stat[4])
        ]);
    };
    table.printstd();
}

pub fn in_service(r: rusoto_core::Region, n: String) -> usize {
    let elb_client = ElbClient::new(r.to_owned());
    let health_params = DescribeEndPointStateInput {
        load_balancer_name: n,
        ..Default::default()
    };

    let instances = elb_client.describe_instance_health(health_params).sync().ok();

    let mut count = 0;
    let instance_states = instances.unwrap().instance_states.unwrap();
    for i in instance_states {
        if i.state == Some("InService".to_string()) {
            count += 1;
        }
    };

    return count as usize
}

pub fn wait_for_in_service(r: rusoto_core::Region, n: String, i: usize, t: u64 ) -> bool {
    let now = Instant::now();
    let mut timer = now.elapsed().as_secs();

    while timer < t {
        let count = in_service(r.clone(), n.clone());

        info!("ELB: {}: want {} InService instances, have {}", n.clone(), i, count);

        if count == i {
            break;
        };

        sleep(Duration::new(15, 0));
        timer = now.elapsed().as_secs();
    };

    if timer >= t {
        warn!("WARN: timeout while waiting for {} instances to join {}", i, n.clone());
        return false
    };

    return true
}
