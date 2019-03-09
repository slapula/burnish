use std::process;
use prettytable::{Table, format};
use rusoto_autoscaling::{
    Autoscaling, AutoscalingClient, AutoScalingGroupNamesType, ScalingProcessQuery,
    CreateAutoScalingGroupType, DeleteAutoScalingGroupType, UpdateAutoScalingGroupType,
    CreateOrUpdateTagsType, TerminateInstanceInAutoScalingGroupType, Instance, Tag,
};

use loadbalancer::wait_for_in_service;

#[derive(Debug, Clone)]
pub struct AutoScaleGroup {
    name:                 std::string::String,
    min_size:             i64,
    max_size:             i64,
    pub desired_capacity: i64,
    pub instance_count:   i64,
    pub lc_name:          std::string::String,
    app_name:             std::string::String,
    env_name:             std::string::String,
}

pub fn list_asg(r: rusoto_core::Region, n: String) -> Vec<AutoScaleGroup> {
    let as_client = AutoscalingClient::new(r.to_owned());
    let mut asg_req = AutoScalingGroupNamesType {
        ..Default::default()
    };

    if n != "" {
		asg_req.auto_scaling_group_names = Some(vec![n])
	}

    let asg_results = as_client.describe_auto_scaling_groups(asg_req).sync().ok();

    let mut app_name = String::new();
    let mut env_name = String::new();
    let mut scaling_groups: Vec<AutoScaleGroup> = Vec::new();

    for asg in asg_results.unwrap().auto_scaling_groups {
        for t in asg.tags.unwrap() {
            if t.key.clone().unwrap() == "app".to_string() {
				app_name = t.value.unwrap()
			} else if t.key.clone().unwrap() == "env".to_string() {
				env_name = t.value.unwrap()
			};
        };

		let scaling_group = AutoScaleGroup {
			name:             asg.auto_scaling_group_name,
			min_size:         asg.min_size,
			max_size:         asg.max_size,
			desired_capacity: asg.desired_capacity,
			instance_count:   asg.instances.unwrap().len() as i64,
			lc_name:          asg.launch_configuration_name.unwrap(),
			app_name:         app_name.clone(),
			env_name:         env_name.clone(),
		};
		scaling_groups.push(scaling_group)
    };

    return scaling_groups
}

pub fn list_asg_cmd(r: rusoto_core::Region, m: &clap::ArgMatches) {
    let results = list_asg(r, m.value_of("name").unwrap().to_string());

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row!["ASG Name", "Instance Count (Current)", "Min Size", "Max Size"]);

    for asg in results {
        table.add_row(row![
            format!("{}", asg.name),
            format!("{}", asg.instance_count),
            format!("{}", asg.min_size),
            format!("{}", asg.max_size)
        ]);
    };
    table.printstd();
}

pub fn create_asg(r: rusoto_core::Region, t: CreateAutoScalingGroupType) {
    let as_client = AutoscalingClient::new(r.to_owned());

    match as_client.create_auto_scaling_group(t.clone()).sync() {
        Ok(_k) => info!("auto scaling group successfully created: {}", t.clone().auto_scaling_group_name),
        Err(error) => panic!("ERROR: {:?}", error),
    };
}

pub fn create_asg_cmd(r: rusoto_core::Region, m: &clap::ArgMatches, u: yaml_rust::Yaml) {
    if u["applications"][m.value_of("app").unwrap()].is_badvalue() {
        panic!("Application {} does not exist in this universe.", m.value_of("app").unwrap().to_string());
    };

    if u["environments"][m.value_of("env").unwrap()].is_badvalue() {
        panic!("Environment {} does not exist in this universe.", m.value_of("env").unwrap().to_string());
    };

    let mut asg_size = 0;
    if m.value_of("canary").unwrap().parse().unwrap() {
        asg_size = 1;
    };

    let app = &u["applications"][m.value_of("app").unwrap()];
    let env = &u["environments"][m.value_of("env").unwrap()];

    let name = format!("{}-{}-{}",
        m.value_of("app").unwrap().to_string(),
        m.value_of("env").unwrap().to_string(),
        m.value_of("version").unwrap().to_string(),
    );

    let elb = app["elb"][m.value_of("env").unwrap()].as_str().unwrap().to_string();
    let service_name = app["service_name"].as_str().unwrap().to_string();
    let mut subnet_ids = Vec::new();

    let subnets = env["subnet_ids"].clone();
    for s in subnets {
        subnet_ids.push(s.as_str().unwrap().to_string())
    };

    let mut tags = Vec::new();
    tags.push(
        Tag {
            key: "Name".to_string(),
            value: Some(name.clone()),
            propagate_at_launch: Some(true),
            ..Default::default()
        }
    );
    tags.push(
        Tag {
            key: "app".to_string(),
            value: Some(m.value_of("app").unwrap().to_string()),
            propagate_at_launch: Some(true),
            ..Default::default()
        }
    );
    tags.push(
        Tag {
            key: "env".to_string(),
            value: Some(m.value_of("env").unwrap().to_string()),
            propagate_at_launch: Some(true),
            ..Default::default()
        }
    );
    tags.push(
        Tag {
            key: "version".to_string(),
            value: Some(m.value_of("version").unwrap().to_string()),
            propagate_at_launch: Some(true),
            ..Default::default()
        }
    );
    tags.push(
        Tag {
            key: "role".to_string(),
            value: Some(m.value_of("role").unwrap().to_string()),
            propagate_at_launch: Some(true),
            ..Default::default()
        }
    );
    tags.push(
        Tag {
            key: "service".to_string(),
            value: Some(service_name),
            propagate_at_launch: Some(true),
            ..Default::default()
        }
    );

    let asg_req = CreateAutoScalingGroupType {
        auto_scaling_group_name: name.clone(),
        desired_capacity: Some(asg_size),
        min_size: asg_size,
        max_size: asg_size,
        health_check_type: Some("ELB".to_string()),
        health_check_grace_period: Some(300),
        launch_configuration_name: Some(m.value_of("launch-config").unwrap().to_string()),
        load_balancer_names: Some(vec![elb]),
        vpc_zone_identifier: Some(subnet_ids.join(",").to_string()),
        tags: Some(tags),
        ..Default::default()
    };

    create_asg(r, asg_req);
}

pub fn destroy_asg(r: rusoto_core::Region, n: String, b: bool) {
    let as_client = AutoscalingClient::new(r.to_owned());
    let asg_req = DeleteAutoScalingGroupType {
        auto_scaling_group_name: n.clone(),
        force_delete: Some(b),
        ..Default::default()
    };

    match as_client.delete_auto_scaling_group(asg_req.clone()).sync() {
        Ok(_k) => info!("auto scaling group successfully destroyed: {}", n.clone()),
        Err(error) => panic!("ERROR: {:?}", error),
    };
}

pub fn destroy_asg_cmd(r: rusoto_core::Region, m: &clap::ArgMatches) {
    destroy_asg(r, m.value_of("name").unwrap().to_string(), m.value_of("force").unwrap().parse().unwrap());
}

pub fn resize_asg(r: rusoto_core::Region, n: String, min: i64, max: i64, d: i64) {
    let as_client = AutoscalingClient::new(r.to_owned());
    let asg_req = UpdateAutoScalingGroupType {
        auto_scaling_group_name: n.clone(),
        min_size: Some(min.clone()),
        max_size: Some(max.clone()),
        desired_capacity: Some(d.clone()),
        ..Default::default()
    };

    match as_client.update_auto_scaling_group(asg_req.clone()).sync() {
        Ok(_k) => info!("auto scaling group successfully resized: {}", n.clone()),
        Err(error) => panic!("ERROR: {:?}", error),
    };
}

pub fn resize_asg_cmd(r: rusoto_core::Region, m: &clap::ArgMatches) {
    resize_asg(
        r, 
        m.value_of("name").unwrap().to_string(),
        m.value_of("min").unwrap().parse().unwrap(),
        m.value_of("max").unwrap().parse().unwrap(),
        m.value_of("desired").unwrap().parse().unwrap(),
    );
}

pub fn rotate_instances(r: rusoto_core::Region, n: String, b: usize) {
    let as_client = AutoscalingClient::new(r.to_owned());
    let mut asg_req = AutoScalingGroupNamesType {
        ..Default::default()
    };

    if n != "" {
		asg_req.auto_scaling_group_names = Some(vec![n.clone()])
	}

    let list_result = as_client.describe_auto_scaling_groups(asg_req).sync().ok().unwrap().auto_scaling_groups;

    if list_result.len() == 0 {
        panic!("ERROR: autoscaling group {} could not be found.", n.clone().to_string());
    }

    if list_result.len() > 1 {
        panic!("ERROR: more than one autoscaling group named {} was found.", n.clone().to_string());
    }

    let asg = list_result.clone();
    let initial_max = asg[0].max_size as usize;
    let initial_desired = asg[0].desired_capacity as usize;

    for (i, sp) in asg[0].suspended_processes.iter().enumerate() {
        let pn = sp[i].clone();
        match pn.process_name.unwrap().as_str() {
            "RemoveFromLoadBalancerLowPriority" | "Terminate" | "Launch" | "HealthCheck" | "AddToLoadBalancer" => {
                panic!("ERROR: {} has a suspended process that must be active", n.clone().to_string());
            }
            _ => ()
        }
    }

    info!("verified {} has correct processes in place", n.clone().to_string());

    let process_req = ScalingProcessQuery {
        auto_scaling_group_name: n.clone(),
        scaling_processes: Some(vec![
            "ReplaceUnhealthy".to_string(),
		    "AlarmNotification".to_string(),
		    "ScheduledActions".to_string(),
		    "AZRebalance".to_string(),
        ]),
    };

    match as_client.suspend_processes(process_req.clone()).sync() {
        Ok(_k) => info!("temporarily suspended ReplaceUnhealthy AlarmNotification ScheduledActions AZRebalance processes for {}", n.clone()),
        Err(error) => panic!("ERROR: {:?}", error),
    };

    let instances = asg[0].instances.clone().unwrap();
    if instances.len() == 0 {
        warn!("WARN: {} has no instances to rotate", n.clone().to_string());
        process::exit(0x0100);
    }

    let mut instances_to_terminate: Vec<Instance> = Vec::new();
    for i in instances {
		if i.lifecycle_state != "InService" {
			info!("ignoring instance {} lifecycle status is: {}", i.instance_id, i.lifecycle_state);
		} else {
			instances_to_terminate.push(i);
		}
    }

    info!("will terminate these instances: {:?}", instances_to_terminate);

	if (initial_desired + b) > initial_max {
		let new_max_size = initial_desired + b;
		let max_size_params = UpdateAutoScalingGroupType {
			auto_scaling_group_name: n.clone(),
			max_size:                Some(new_max_size as i64),
            ..Default::default()
		};

        match as_client.update_auto_scaling_group(max_size_params).sync() {
            Ok(_k) => info!("updating max size to {}", new_max_size),
            Err(error) => panic!("ERROR: {:?}", error),
        };
	}

	let new_desired_capacity = initial_desired + b;
	let capacity_params = UpdateAutoScalingGroupType {
		auto_scaling_group_name: n.clone(),
		desired_capacity:        Some(new_desired_capacity as i64),
        ..Default::default()
	};

    match as_client.update_auto_scaling_group(capacity_params).sync() {
        Ok(_k) => info!("resizing {} to new desired size: {}", n.clone(), new_desired_capacity),
        Err(error) => panic!("ERROR: {:?}", error),
    };

	let elbs = asg[0].load_balancer_names.clone().unwrap();

	info!("starting to cull old instances");

    for e in elbs.iter().enumerate() {
        let (mut i, item): (usize, &std::string::String) = e;
        let mut _count = i += b;
        let mut max = i + b;
        if max > instances_to_terminate.len() {
            max = instances_to_terminate.len();
        };

        if !wait_for_in_service(r.clone(), item.to_string(), new_desired_capacity, 60 * 15) {
            panic!("ERROR: timed out waiting for instnaces to register with the ELB")
        };

		for inst in instances_to_terminate[i..max].to_vec() {
			info!("starting to remove instance: {}", inst.instance_id);

            let term_inst_params = TerminateInstanceInAutoScalingGroupType {
                instance_id: inst.instance_id.clone(),
                should_decrement_desired_capacity: false,
            };

            match as_client.terminate_instance_in_auto_scaling_group(term_inst_params).sync() {
                Ok(_k) => info!("instance {} has been terminated", inst.instance_id.clone()),
                Err(error) => panic!("ERROR: {:?}", error),
            };
		};
    };

    info!("instance rotation is complete");

    let reset_params = UpdateAutoScalingGroupType {
        auto_scaling_group_name: n.clone(),
        max_size:                Some(initial_max as i64),
        desired_capacity:        Some(initial_desired as i64),
        ..Default::default()
    };

    match as_client.update_auto_scaling_group(reset_params).sync() {
        Ok(_k) => info!("resized asg to previous size. max: {} desired: {}", initial_max, initial_desired),
        Err(error) => panic!("ERROR: {:?}", error),
    };

    match as_client.resume_processes(process_req.clone()).sync() {
        Ok(_k) => info!("resumed ReplaceUnhealthy AlarmNotification ScheduledActions AZRebalance processes for {}", n.clone()),
        Err(error) => panic!("ERROR: {:?}", error),
    };
}

pub fn rotate_instances_cmd(r: rusoto_core::Region, m: &clap::ArgMatches, u: yaml_rust::Yaml) {
    rotate_instances(r, m.value_of("name").unwrap().to_string(), m.value_of("batch").unwrap().parse::<usize>().unwrap());
}

pub fn updatelc_asg(r: rusoto_core::Region, n: String, lc: String) {
    let as_client = AutoscalingClient::new(r.to_owned());
    let asg_req = UpdateAutoScalingGroupType {
        auto_scaling_group_name: n.clone(),
        launch_configuration_name: Some(lc.clone()),
        ..Default::default()
    };

    match as_client.update_auto_scaling_group(asg_req.clone()).sync() {
        Ok(_k) => info!("launch configuration successfully updated: {}", n.clone()),
        Err(error) => panic!("ERROR: {:?}", error),
    };
}

pub fn updatelc_asg_cmd(r: rusoto_core::Region, m: &clap::ArgMatches) {
    updatelc_asg(
        r, 
        m.value_of("name").unwrap().to_string(),
        m.value_of("launch-config").unwrap().to_string(),
    );
}

pub fn update_version_tag(r: rusoto_core::Region, n: String, v: String) {
    let as_client = AutoscalingClient::new(r.to_owned());
    let tag = Tag {
        key:                 "version".to_string(),
        propagate_at_launch: Some(true),
        resource_id:         Some(n.clone()),
        resource_type:       Some("auto-scaling-group".to_string()),
        value:               Some(v),
    };
    let asg_tag_req = CreateOrUpdateTagsType {
        tags: vec![tag],
        ..Default::default()
    };

    match as_client.create_or_update_tags(asg_tag_req).sync() {
        Ok(_k) => info!("version tag successfully updated: {}", n.clone()),
        Err(error) => panic!("ERROR: {:?}", error),
    };
}