extern crate rusoto_core;

use rusoto_core::Region;

pub fn parse_region(s: &str) -> Result<Region, Region> {
    let v : &str = &s.to_lowercase();
    match v {
        "ap-northeast-1" | "apnortheast1" => Ok(Region::ApNortheast1),
        "ap-northeast-2" | "apnortheast2" => Ok(Region::ApNortheast2),
        "ap-south-1" | "apsouth1" => Ok(Region::ApSouth1),
        "ap-southeast-1" | "apsoutheast1" => Ok(Region::ApSoutheast1),
        "ap-southeast-2" | "apsoutheast2" => Ok(Region::ApSoutheast2),
        "ca-central-1" | "cacentral1" => Ok(Region::CaCentral1),
        "eu-central-1" | "eucentral1" => Ok(Region::EuCentral1),
        "eu-west-1" | "euwest1" => Ok(Region::EuWest1),
        "eu-west-2" | "euwest2" => Ok(Region::EuWest2),
        "eu-west-3" | "euwest3" => Ok(Region::EuWest3),
        "sa-east-1" | "saeast1" => Ok(Region::SaEast1),
        "us-east-1" | "useast1" => Ok(Region::UsEast1),
        "us-east-2" | "useast2" => Ok(Region::UsEast2),
        "us-west-1" | "uswest1" => Ok(Region::UsWest1),
        "us-west-2" | "uswest2" => Ok(Region::UsWest2),
        "us-gov-west-1" | "usgovwest1" => Ok(Region::UsGovWest1),
        "cn-north-1" | "cnnorth1" => Ok(Region::CnNorth1),
        "cn-northwest-1" | "cnnorthwest1" => Ok(Region::CnNorthwest1),
        _s => Err(Region::UsEast1),
    }
}
