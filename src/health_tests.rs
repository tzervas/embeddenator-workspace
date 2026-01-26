//! Tests for workspace health checking.

#[cfg(test)]
mod tests {
    use crate::{HealthCheckType, HealthChecker, HealthStatus};
    use std::fs;
    use tempfile::TempDir;

    fn create_test_workspace() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a mock embeddenator-* package
        let pkg1 = root.join("embeddenator-test1");
        fs::create_dir_all(&pkg1).unwrap();

        fs::write(
            pkg1.join("Cargo.toml"),
            r#"
[package]
name = "embeddenator-test1"
version = "0.20.0-alpha.1"
edition = "2021"

[dependencies]
            "#,
        )
        .unwrap();

        // Create lib.rs
        fs::create_dir_all(pkg1.join("src")).unwrap();
        fs::write(pkg1.join("src/lib.rs"), "pub fn test() {}").unwrap();

        // Create specs directory
        let specs_dir = pkg1.join("specs");
        fs::create_dir_all(&specs_dir).unwrap();
        fs::write(specs_dir.join("spec.md"), "# Test Spec").unwrap();

        // Create second package without specs
        let pkg2 = root.join("embeddenator-test2");
        fs::create_dir_all(&pkg2).unwrap();

        fs::write(
            pkg2.join("Cargo.toml"),
            r#"
[package]
name = "embeddenator-test2"
version = "0.20.0-alpha.1"
edition = "2021"
            "#,
        )
        .unwrap();

        fs::create_dir_all(pkg2.join("src")).unwrap();
        fs::write(pkg2.join("src/lib.rs"), "pub fn test2() {}").unwrap();

        temp_dir
    }

    #[tokio::test]
    async fn test_health_checker_creation() {
        let temp_dir = create_test_workspace();
        let _checker = HealthChecker::new(temp_dir.path());

        // Just verify it can be created
        assert_eq!(temp_dir.path(), temp_dir.path());
    }

    #[tokio::test]
    async fn test_spec_coverage_check() {
        let temp_dir = create_test_workspace();
        let checker = HealthChecker::new(temp_dir.path());

        let check_types = vec![HealthCheckType::Specs];
        let report = checker.check_selected(&check_types, false).await.unwrap();

        assert_eq!(report.checks.len(), 1);
        let spec_check = &report.checks[0];

        assert_eq!(spec_check.check_type, HealthCheckType::Specs);
        // Should be warning because one package has specs, one doesn't
        assert_eq!(spec_check.status, HealthStatus::Warn);
        assert!(spec_check.message.contains("50.0%") || spec_check.message.contains("coverage"));
    }

    #[tokio::test]
    async fn test_version_check() {
        let temp_dir = create_test_workspace();
        let checker = HealthChecker::new(temp_dir.path());

        let check_types = vec![HealthCheckType::Version];
        let report = checker.check_selected(&check_types, false).await.unwrap();

        assert_eq!(report.checks.len(), 1);
        let version_check = &report.checks[0];

        assert_eq!(version_check.check_type, HealthCheckType::Version);
        // Should pass since both packages have consistent versions
        assert_eq!(version_check.status, HealthStatus::Pass);
    }

    #[tokio::test]
    async fn test_parallel_checks() {
        let temp_dir = create_test_workspace();
        let checker = HealthChecker::new(temp_dir.path());

        // Run multiple checks in parallel
        let check_types = vec![HealthCheckType::Version, HealthCheckType::Specs];

        let report = checker.check_selected(&check_types, false).await.unwrap();

        // Should have results for both checks
        assert_eq!(report.checks.len(), 2);

        // Verify both check types are present
        let check_type_set: std::collections::HashSet<_> =
            report.checks.iter().map(|c| c.check_type).collect();
        assert!(check_type_set.contains(&HealthCheckType::Version));
        assert!(check_type_set.contains(&HealthCheckType::Specs));
    }

    #[tokio::test]
    async fn test_report_has_failures() {
        let temp_dir = create_test_workspace();
        let checker = HealthChecker::new(temp_dir.path());

        let report = checker.check_all(false).await.unwrap();

        // We can't guarantee failures in the mock workspace,
        // but we can verify the method works
        let _ = report.has_failures();
    }

    #[tokio::test]
    async fn test_markdown_generation() {
        let temp_dir = create_test_workspace();
        let checker = HealthChecker::new(temp_dir.path());

        let check_types = vec![HealthCheckType::Version];
        let report = checker.check_selected(&check_types, false).await.unwrap();

        let markdown = report.to_markdown();

        // Verify markdown structure
        assert!(markdown.contains("# Workspace Health Report"));
        assert!(markdown.contains("**Generated:**"));
        assert!(markdown.contains("**Workspace:**"));
        assert!(markdown.contains("**Overall Status:**"));
        assert!(markdown.contains("## Check Results"));
    }

    #[tokio::test]
    async fn test_json_serialization() {
        let temp_dir = create_test_workspace();
        let checker = HealthChecker::new(temp_dir.path());

        let check_types = vec![HealthCheckType::Version];
        let report = checker.check_selected(&check_types, false).await.unwrap();

        // Should be able to serialize to JSON
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("timestamp"));
        assert!(json.contains("workspace_root"));
        assert!(json.contains("checks"));
        assert!(json.contains("overall_status"));
    }

    #[test]
    fn test_health_check_type_from_str() {
        assert_eq!("git".parse::<HealthCheckType>(), Ok(HealthCheckType::Git));
        assert_eq!(
            "version".parse::<HealthCheckType>(),
            Ok(HealthCheckType::Version)
        );
        assert_eq!(
            "tests".parse::<HealthCheckType>(),
            Ok(HealthCheckType::Tests)
        );
        assert_eq!("docs".parse::<HealthCheckType>(), Ok(HealthCheckType::Docs));
        assert_eq!(
            "specs".parse::<HealthCheckType>(),
            Ok(HealthCheckType::Specs)
        );
        assert!("invalid".parse::<HealthCheckType>().is_err());
    }

    #[test]
    fn test_health_status_is_critical() {
        assert!(!HealthStatus::Pass.is_critical());
        assert!(!HealthStatus::Warn.is_critical());
        assert!(HealthStatus::Fail.is_critical());
    }
}
