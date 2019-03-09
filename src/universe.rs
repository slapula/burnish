use yaml_rust::yaml;
use http::Uri;
use futures::{Stream, Future};
use rusoto_s3::{S3, S3Client, GetObjectRequest};

pub fn get_universe (u: std::string::String, r: rusoto_core::Region) -> yaml_rust::Yaml {
    if u.starts_with("s3://") {
        let s3_client = S3Client::new(r.to_owned());
        let uri = u.parse::<Uri>().unwrap();
        let get_req = GetObjectRequest {
            bucket: uri.host().unwrap().to_string(),
            key: uri.path().to_string(),
            ..Default::default()
        };
        let result = s3_client.get_object(get_req).sync().expect("Couldn't GET universe file from S3");
        let stream = result.body.unwrap();
        let body: Vec<u8> = stream.concat2().wait().unwrap();
        let universes = yaml::YamlLoader::load_from_str(&String::from_utf8_lossy(&body)).unwrap();
        let universe = universes[0].clone();
        return universe;
    } else {
        let f = std::fs::read_to_string(u).expect("failed to open local universe file");
        let universes = yaml::YamlLoader::load_from_str(&f).unwrap();
        let universe = universes[0].clone();
        return universe;
    }
}