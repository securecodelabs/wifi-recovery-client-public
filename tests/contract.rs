use wifi_recovery_client::contract;

#[test]
fn accepts_shared_contract_and_rejects_unknown_state_or_invalid_uuid() {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/job-v1.json")).unwrap();
    let job = contract::job(&value).unwrap();
    assert_eq!(job.search.pattern, "Home?d?d?d?d");
    assert_eq!(job.state.watch_exit(), None);
    value["state"] = "unknown".into();
    assert!(contract::job(&value).is_err());
    value["state"] = "stopped".into();
    assert_eq!(contract::job(&value).unwrap().state.watch_exit(), Some(11));
    value["id"] = "wrong".into();
    assert!(contract::job(&value).is_err());
}

#[test]
fn result_validation_rejects_invalid_record_counts_and_unverified_passwords() {
    let id = uuid::Uuid::new_v4();
    let mut result = serde_json::json!({"contract":1,"job_id":id,"engine_result":{
        "schema":1,"total_records":1,"recovered":1,
        "matches":[{"verified":true,"password_hex":hex::encode("password")}]}});
    assert!(wifi_recovery_client::output::validate_result(&result, id).is_ok());
    for total in [0, 10001] {
        result["engine_result"]["total_records"] = total.into();
        assert!(wifi_recovery_client::output::validate_result(&result, id).is_err());
    }
    result["engine_result"]["total_records"] = 1.into();
    result["engine_result"]["matches"][0]["verified"] = false.into();
    assert!(wifi_recovery_client::output::validate_result(&result, id).is_err());
}
