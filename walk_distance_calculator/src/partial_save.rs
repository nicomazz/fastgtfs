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
   let tuple_keyed_result = HashMap::<(usize,usize), usize>::deserialize(r).unwrap();
   tuple_keyed_result
       .into_iter()
       .map(|((a,b), v)| (StopPair { a, b }, v))
       .collect()
}



pub(crate) fn save_partial_data(res: &DistancesResult) {
    let mut buffer = flexbuffers::FlexbufferSerializer::new();
    let res_with_string_key = res
        .iter()
        .map(|(k, v)| ((k.a,k.b), *v))
        .collect::<HashMap<(usize,usize), usize>>();
    res_with_string_key.serialize(&mut buffer).unwrap();
    let mut output_file =
        File::create(TEMP_FILE_NAME).unwrap_or_else(|_| panic!("Can't create {}", TEMP_FILE_NAME));
    output_file.write_all(buffer.view()).unwrap();
}