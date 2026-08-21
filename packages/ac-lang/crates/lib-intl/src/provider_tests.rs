use super::module::IntlModule;
use super::provider::IntlProvider;
use ac_runtime_core::Module;

#[test]
fn provider_from_blob() {
    let provider = IntlProvider::from_blob(vec![0u8; 10]);
    assert_eq!(provider.as_bytes().len(), 10);
}

#[test]
fn module_new_has_no_provider() {
    let module = IntlModule::new();
    assert!(module.provider().is_none());
}

#[test]
fn module_with_blob_provider_has_provider() {
    let module = IntlModule::with_blob_provider(vec![0u8; 10]);
    assert!(module.provider().is_some());
    assert_eq!(module.provider().unwrap().as_bytes().len(), 10);
}

#[test]
fn module_with_provider_still_resolves_functions() {
    let module = IntlModule::with_blob_provider(vec![0u8; 10]);
    assert!(module.function("format_number").is_some());
    assert!(module.function("select_plural").is_some());
    assert!(module.function("format_list").is_some());
    assert!(module.function("format_relative_time").is_some());
    assert!(module.function("format_date").is_some());
    assert!(module.function("format_duration").is_some());
    assert!(module.function("compare_strings").is_some());
    assert!(module.function("display_name").is_some());
    assert!(module.function("segment").is_some());
}

#[test]
fn module_default_is_new() {
    let module = IntlModule::default();
    assert!(module.provider().is_none());
}
