use ubuntu_custom_image::tools::preflight;

#[test]
fn preflight_reports_unsupported_or_missing_environment_without_panicking() {
    let result = preflight();
    if cfg!(target_os = "linux") {
        let _ = result;
    } else {
        assert!(result.is_err());
    }
}
