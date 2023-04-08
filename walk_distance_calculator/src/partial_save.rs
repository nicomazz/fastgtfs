use std::{path::Path, fs::File, io::Write, collections::HashMap};

use fastgtfs::raw_parser::read_file;

use crate::{DistancesResult, TEMP_FILE_NAME, StopPair};
use serde::{Deserialize, Serialize};


pub(crate) fn restore_partial_data() -> DistancesResult {
   let path = Path::new(TEMP_FILE_NAME);
   if !path.exists() {
       return DistancesResult::new();
   }

   let content = read_file(path);
   let r = flexbuffers::Reader::get_root(&content).unwrap();
   let string_keyed_result = HashMap::<String, usize>::deserialize(r).unwrap();
   string_keyed_result
       .into_iter()
       .map(|(k, v)| (stop_pair_from_string(&k), v))
       .collect()
}


fn stop_pair_to_string(pair: &StopPair) -> String {
   format!("{}-{}", pair.a, pair.b)
}

fn stop_pair_from_string(s: &str) -> StopPair {
   let mut split = s.split('-');
   let a = split.next().unwrap().parse().unwrap();
   let b = split.next().unwrap().parse().unwrap();
   StopPair { a, b }
}

pub(crate) fn save_partial_data(res: &DistancesResult) {
   let mut buffer = flexbuffers::FlexbufferSerializer::new();
   let res_with_string_key = res
       .iter()
       .map(|(k, v)| (stop_pair_to_string(k), *v))
       .collect::<HashMap<String, usize>>();
      res_with_string_key.serialize(&mut buffer).unwrap();
   let mut output_file =
       File::create(TEMP_FILE_NAME).unwrap_or_else(|_| panic!("Can't create {}", TEMP_FILE_NAME));
   output_file.write_all(buffer.view()).unwrap();
}