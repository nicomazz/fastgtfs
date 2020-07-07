pub fn get_test_paths() -> Vec<String> {
    ["actv_aut", "actv_nav", "alilaguna"]
        .iter()
        .map(|s| format!("./test_data/{}", s.to_owned()))
        .collect::<Vec<String>>()
}
