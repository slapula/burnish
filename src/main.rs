#[macro_use] extern crate log;
#[macro_use] extern crate clap;
#[macro_use] extern crate prettytable;
extern crate http;
extern crate futures;
extern crate chrono;
extern crate timeago;
extern crate yaml_rust;
extern crate rusoto_core;
extern crate rusoto_s3;
extern crate rusoto_ec2;
extern crate rusoto_elb;
extern crate rusoto_autoscaling;
extern crate rusoto_cloudwatch;

use clap::App;

pub mod utils;
pub mod universe;
pub mod oneoff;
pub mod launchconfig;
pub mod loadbalancer;
pub mod autoscalegroup;
pub mod deployment;

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let universe_file = matches.value_of("universe").unwrap_or("universe.yml");

    // NOTE: Rusoto behavior is to take malformed/non-existant
    // regions and default to `us-east-1`.  I am mirroring that behavior here.
    let region_val = matches.value_of("region").unwrap_or("us-east-1");
    let region_result = utils::parse_region(region_val);
    let region = match region_result {
        Ok(v) => v,
        Err(e) => e,
    };

    let universe = universe::get_universe(universe_file.to_string(), region.clone());

    if let Some(matches) = matches.subcommand_matches("oneoff") {
        if let Some(sub_m) = matches.subcommand_matches("launch") {
            oneoff::launch_instance(sub_m, region.clone());
        };
        if let Some(sub_m) = matches.subcommand_matches("terminate") {
            oneoff::term_instances(sub_m, region.clone());
        };
    };

    if let Some(matches) = matches.subcommand_matches("launchconfig") {
        launchconfig::create_lc(region.clone(), matches, universe.clone());
    };

    if let Some(matches) = matches.subcommand_matches("loadbalancer") {
        if let Some(sub_m) = matches.subcommand_matches("stats") {
            loadbalancer::elb_stats_cmd(region.clone(), sub_m);
        };
        if let Some(sub_m) = matches.subcommand_matches("status") {
            loadbalancer::elb_status(sub_m, region.clone());
        };
    };

    if let Some(matches) = matches.subcommand_matches("autoscalegroup") {
        if let Some(sub_m) = matches.subcommand_matches("list") {
            autoscalegroup::list_asg_cmd(region.clone(), sub_m);
        };
        if let Some(sub_m) = matches.subcommand_matches("create") {
            autoscalegroup::create_asg_cmd(region.clone(), sub_m, universe.clone());
        };
        if let Some(sub_m) = matches.subcommand_matches("destroy") {
            autoscalegroup::destroy_asg_cmd(region.clone(), sub_m);
        };
        if let Some(sub_m) = matches.subcommand_matches("resize") {
            autoscalegroup::resize_asg_cmd(region.clone(), sub_m);
        };
        if let Some(sub_m) = matches.subcommand_matches("rotate") {
            autoscalegroup::rotate_instances_cmd(region.clone(), sub_m, universe.clone());
        };
        if let Some(sub_m) = matches.subcommand_matches("updatelc") {
            autoscalegroup::updatelc_asg_cmd(region.clone(), sub_m);
        };
    };

    if let Some(matches) = matches.subcommand_matches("deployment") {
        if let Some(sub_m) = matches.subcommand_matches("do") {
            deployment::do_deployment_cmd(region.clone(), sub_m, universe.clone());
        };
        //if let Some(sub_m) = matches.subcommand_matches("mark") {
        //    deployment::mark_deployment_cmd(region.clone(), sub_m, universe.clone());
        //};
    };

}
