use std::{thread, time};
use launchconfig::create_lc;
use loadbalancer::{elb_stats, in_service, wait_for_in_service};
use autoscalegroup::{list_asg, resize_asg, rotate_instances, updatelc_asg, update_version_tag};

#[derive(Debug, Clone)]
pub struct Deployment {
	application:            std::string::String,
	environment:            std::string::String,
	iam_profile:            std::string::String,
	instance_type:          std::string::String,
	version:                std::string::String,
	force:                  bool,
	max_latency:            f64,
	max_error_rate:         f64,
	user_data:              std::string::String,
	healthcheck_timeout:    u64,
    strategy:               std::string::String,
	batch:                  usize,
}

pub fn do_deployment_cmd(r: rusoto_core::Region, m: &clap::ArgMatches, u: yaml_rust::Yaml) {
    if u["applications"][m.value_of("app").unwrap()].is_badvalue() {
        panic!("Application {} does not exist in this universe.", m.value_of("app").unwrap().to_string());
    };

    if u["environments"][m.value_of("env").unwrap()].is_badvalue() {
        panic!("Environment {} does not exist in this universe.", m.value_of("env").unwrap().to_string());
    };

    if m.value_of("version").unwrap().to_string().chars().count() > 255 {
        panic!("max length for version string is 255 chars.");
    };

    let deploy = Deployment {
        application:            m.value_of("app").unwrap().to_string(),
        environment:            m.value_of("env").unwrap().to_string(),
        iam_profile:            m.value_of("iam-profile").unwrap().to_string(),
        instance_type:          m.value_of("instance-type").unwrap().to_string(),
        version:                m.value_of("version").unwrap().to_string(),
        force:                  m.value_of("force").unwrap().parse::<bool>().unwrap(),
        max_latency:            m.value_of("max-latency").unwrap().parse::<f64>().unwrap() / 1000.0,
        max_error_rate:         m.value_of("max-error-rate").unwrap().parse::<f64>().unwrap() / 100.0,
        user_data:              m.value_of("user-data").unwrap().to_string(),
        healthcheck_timeout:    m.value_of("timeout").unwrap().parse::<u64>().unwrap(),
        strategy:               m.value_of("strategy").unwrap().to_string(),
        batch:                  m.value_of("batch").unwrap().parse::<usize>().unwrap(),
    };

    let app = &u["applications"][m.value_of("app").unwrap()];
    let env = &u["environments"][m.value_of("env").unwrap()];
    let lc = create_lc(r.clone(), m, u.clone());
    let elb = app["elb"][m.value_of("env").unwrap()].as_str().unwrap().to_string();
    let blue_asg = format!(
        "{}-{}-blue",
        m.value_of("app").unwrap().to_string(),
        m.value_of("env").unwrap().to_string(),
    );
    let green_asg = format!(
        "{}-{}-green",
        m.value_of("app").unwrap().to_string(),
        m.value_of("env").unwrap().to_string(),
    );

    if !deploy.force {
        let initial_stats = elb_stats(r.clone(), elb.clone(), 5);

        info!("established baseline performance stats: {:?}", initial_stats);
        
        let bsg = list_asg(r.clone(), blue_asg.clone());
        if bsg.len() == 0 {
            panic!("there is a problem getting the auto scaling group information");
        };

        let blue_asg_info = bsg[0].clone();

		if blue_asg_info.instance_count != 0 || blue_asg_info.desired_capacity != 0 {
			info!("current instance count is {} and desired capacity is {}", blue_asg_info.instance_count, blue_asg_info.desired_capacity);
			panic!("ERROR: blue ASG is not set to 0 instances. Is there another deploy happening?");
		};

        updatelc_asg(r.clone(), blue_asg.clone(), lc.clone());
        update_version_tag(r.clone(), blue_asg.clone(), deploy.clone().version);

        let in_service = in_service(r.clone(), elb.clone());

        resize_asg(r.clone(), blue_asg.clone(), 1, 1, 1);

        info!("resized blue asg to launch a canary instance, waiting for canary to enter load...");
        if !wait_for_in_service(r.clone(), blue_asg.clone(), in_service + 1, deploy.healthcheck_timeout * 60) {
            resize_asg(r.clone(), blue_asg.clone(), 0, 0, 0);
            info!("resized blue asg to: {}", blue_asg_info.desired_capacity);

            updatelc_asg(r.clone(), blue_asg.clone(), blue_asg_info.clone().lc_name);
            info!("reset launch config to original value: {}", blue_asg_info.clone().lc_name);

            panic!("ERROR: timed out waiting for the canary to register with the ELB");
        };

        info!("canary instance is registered with the ELB and taking traffic. starting a 5 minute monitoring window.");
        for _s in 1..5 {
            let mut canary_wait_stats = elb_stats(r.clone(), elb.clone(), 1);
            info!("stats: {:?}", canary_wait_stats);

            thread::sleep(time::Duration::from_secs(60));
        };

        let canary_stats = elb_stats(r.clone(), elb.clone(), 5);
        info!("canary stats (5 min): {:?}", canary_stats);

        if !deploy.force {
            info!("error rate: {:.50} max error rate: {:.50}", canary_stats[3], deploy.max_error_rate);

            if canary_stats[3] > deploy.max_error_rate {
                resize_asg(r.clone(), blue_asg.clone(), 0, 0, 0);
                info!("error rate exceeded MaxErrorRate");
            };

            info!("latency: {:.50} max allowed latency: {:.50}", canary_stats[4], deploy.max_latency);

            if canary_stats[4] > deploy.max_latency {
                resize_asg(r.clone(), blue_asg.clone(), 0, 0, 0);
                info!("request latency exceeded MaxLatency");
            };

            info!("canary stats are good. will remove canary and rotate instances");
        } else {
            info!("skipping error and latency checks because this is a force deploy");
        };
    };

    let gsg = list_asg(r.clone(), green_asg.clone());
    if gsg.len() == 0 {
        panic!("there is a problem getting the auto scaling group information");
    };

    let green_asg_info = gsg[0].clone();

    updatelc_asg(r.clone(), green_asg.clone(), lc.clone());
    update_version_tag(r.clone(), green_asg.clone(), deploy.clone().version);

    info!("will now rotate all instances in green ASG...");
    if deploy.strategy == "replacement".to_string() {
        rotate_instances(r.clone(), green_asg.clone(), green_asg_info.instance_count as usize);
    } else {
        rotate_instances(r.clone(), green_asg.clone(), deploy.batch);
    };

    info!("rotated instances in the green ASG");
}