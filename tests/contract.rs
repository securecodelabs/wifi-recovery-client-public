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
