pub mod here_api {
    use std::{env, io::Read};

    use fastgtfs::gtfs_data::LatLng;
    use itertools::Itertools;
    use serde_json::Value;

    use crate::RResult;

    pub(crate) fn here_distance_request(
        from: LatLng,
        tos: Vec<LatLng>,
    ) -> RResult<Vec<Option<usize>>> {
        let api_key = env::var("HERE_APIKEY").expect("There is no HERE API key set as env var. Please, set it (it's free for a lot of requests).");

        let start_get_param = format!("start0={},{}", from.lat, from.lng);
        let destinations_get_param = tos
            .iter()
            .enumerate()
            .map(|(i, p)| format!("destination{}={},{}", i, p.lat, p.lng))
            .join("&");
        let request_url = format!(
            "https://matrix.route.ls.hereapi.com/routing/7.2\
       /calculatematrix.json?\
       apikey={}&\
       mode=fastest;pedestrian;boatFerry:-3&\
       {}&\
       {}",
            api_key, start_get_param, destinations_get_param
        );

        let body = do_request(request_url)?;
        // print the response:
        let json_reply: Value = serde_json::from_str(&body).unwrap();
        //if there is an error key, return error
        if json_reply["error"].is_null() == false {
            let error_message = json_reply["error"].to_string();
            println!("Error: {}", error_message);
            return Ok(vec![]);
            //return Err(RequestError::CustomError(format!("Error message: {}", error_message)));
        }

        let entries = json_reply["response"]["matrixEntry"].as_array().unwrap();
        assert_eq!(entries.len(), tos.len());

        Ok(entries
            .into_iter()
            .map(|e| e.as_object().unwrap())
            .sorted_by_key(|e| e["destinationIndex"].as_i64().unwrap())
            .map(|e| {
                if e.contains_key("summary") {
                    e["summary"]["costFactor"].as_i64()
                } else {
                    None
                }
            })
            .map(|e| match e {
                None => None,
                Some(s) => Some(s as usize),
            })
            .collect())
    }
    fn do_request(url: String) -> RResult<String> {
        let mut res = reqwest::blocking::get(&url)?;
        let mut body = String::new();
        res.read_to_string(&mut body).unwrap();
        Result::Ok(body)
    }
}
