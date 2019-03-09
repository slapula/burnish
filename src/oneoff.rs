use rusoto_ec2::{Ec2, Ec2Client, Tag, TagSpecification,
    IamInstanceProfileSpecification, RunInstancesRequest,
    TerminateInstancesRequest};

pub fn launch_instance(m: &clap::ArgMatches, r: rusoto_core::Region) {
    let ec2_client = Ec2Client::new(r.to_owned());
    let iam_profile = IamInstanceProfileSpecification {
        name: Some(m.value_of("iam-profile").unwrap().to_string()),
        ..Default::default()
    };
    let name_tag = Tag {
        key: Some("name".to_string()),
        value: Some(m.value_of("name").unwrap().to_string()),
    };
    let env_tag = Tag {
        key: Some("env".to_string()),
        value: Some(m.value_of("env").unwrap().to_string()),
    };
    let tag_spec = TagSpecification {
        tags: Some(vec![name_tag, env_tag]),
        ..Default::default()
    };
    let run_req = RunInstancesRequest {
        image_id: Some(m.value_of("ami").unwrap().to_string()),
        key_name: Some(m.value_of("key").unwrap().to_string()),
        instance_type: Some(m.value_of("instance-type").unwrap().to_string()),
        iam_instance_profile: Some(iam_profile),
        security_group_ids: Some(m.value_of("security-groups").unwrap().split(",").map(|s| s.to_string()).collect() ),
        tag_specifications: Some(vec![tag_spec]),
        ..Default::default()
    };
    match ec2_client.run_instances(run_req).sync() {
        Ok(_a) => println!("SUCCESS: Instance successfully launched"),
        Err(error) => eprintln!("ERROR: {:?}", error),
    };
}

pub fn term_instances(m: &clap::ArgMatches, r: rusoto_core::Region) {
    let ec2_client = Ec2Client::new(r.to_owned());
    let term_req = TerminateInstancesRequest {
        instance_ids: m.value_of("instanceids").unwrap().split(",").map(|s| s.to_string()).collect(),
        ..Default::default()
    };
    match ec2_client.terminate_instances(term_req).sync() {
        Ok(_a) => println!("SUCCESS: Instance(s) successfully terminated"),
        Err(error) => eprintln!("ERROR: {:#?}", error),
    };
}

                    // - foreground:
                    //     help: Launch an instance and send sigint to terminal to terminate
                    //     short: f
                    //     long: foreground