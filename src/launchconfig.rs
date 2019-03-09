use chrono::prelude::*;
use rusoto_autoscaling::{Autoscaling, AutoscalingClient, CreateLaunchConfigurationType};

pub fn create_lc(r: rusoto_core::Region, m: &clap::ArgMatches, u: yaml_rust::Yaml) -> String {
    let as_client = AutoscalingClient::new(r.to_owned());

    if u["applications"][m.value_of("app").unwrap()].is_badvalue() {
        panic!("Application {} does not exist in this universe.", m.value_of("app").unwrap().to_string());
    };

    if u["environments"][m.value_of("env").unwrap()].is_badvalue() {
        panic!("Environment {} does not exist in this universe.", m.value_of("env").unwrap().to_string());
    };

    let app = &u["applications"][m.value_of("app").unwrap()];
    let security_groups = app["security_groups"][m.value_of("env").unwrap()].clone();
    let mut sg_ids = Vec::new();
    for s in security_groups {
        sg_ids.push(s.as_str().unwrap().to_string())
    };

    let lc_name = format!("{}-{}-{}-{}",
        m.value_of("app").unwrap().to_string(),
        m.value_of("env").unwrap().to_string(),
        m.value_of("version").unwrap().to_string(),
        Utc::today().format("%Y%m%d%H%M%S").to_string()
    );
    let lc_req = CreateLaunchConfigurationType {
        launch_configuration_name: lc_name.clone(),
        image_id: Some(m.value_of("ami").unwrap().to_string()),
        instance_type: Some(m.value_of("instance-type").unwrap().to_string()),
        iam_instance_profile: Some(m.value_of("iam-profile").unwrap().to_string()),
        user_data: Some(m.value_of("user-data").unwrap().to_string()),
        security_groups: Some(sg_ids),
        ..Default::default()
    };
    match as_client.create_launch_configuration(lc_req).sync() {
        Ok(_a) => info!("launch configuration {} successfully created", lc_name.clone().to_string()),
        Err(error) => eprintln!("ERROR: {:?}", error),
    };

    return lc_name
}