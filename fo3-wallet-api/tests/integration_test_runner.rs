//! Integration Test Runner
//! 
//! Main test runner for Phase 3 Integration Testing & Quality Assurance
//! Orchestrates all validation tests and generates comprehensive reports

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio;
use chrono::Utc;
use serde::{Deserialize, Serialize};

mod e2e_test_framework;
mod performance_validation;
mod security_validation;
mod service_registration_validation;

use e2e_test_framework::{E2ETestFramework, TestScenarioResult};
use performance_validation::{PerformanceValidator, PerformanceTestConfig, PerformanceTestResult};
use security_validation::{SecurityValidator, SecurityTestConfig, SecurityTestResult};
use service_registration_validation::{ServiceRegistrationValidator, ServiceRegistrationResult, ValidationReport};

use fo3_wallet_api::state::AppState;
use fo3_wallet_api::middleware::{
    auth::AuthService,
    audit::AuditLogger,
    rate_limit::RateLimiter,
    trading_guard::TradingGuard,
};
use fo3_wallet_api::ml::ModelManager;
use fo3_wallet_api::services::automated_trading::AutomatedTradingServiceImpl;

/// Integration test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationTestConfig {
    pub server_address: String,
    pub test_timeout_seconds: u64,
    pub parallel_execution: bool,
    pub generate_reports: bool,
    pub report_output_dir: String,
    pub performance_config: PerformanceTestConfig,
    pub security_config: SecurityTestConfig,
}

/// Comprehensive test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveTestResults {
    pub test_run_id: String,
    pub started_at: chrono::DateTime<Utc>,
    pub completed_at: chrono::DateTime<Utc>,
    pub total_duration: Duration,
    pub overall_success: bool,
    pub e2e_results: Vec<TestScenarioResult>,
    pub performance_results: Vec<PerformanceTestResult>,
    pub security_results: Vec<SecurityTestResult>,
    pub service_registration_results: Vec<ServiceRegistrationResult>,
    pub validation_report: ValidationReport,
    pub summary: TestSummary,
    pub recommendations: Vec<String>,
}

/// Test summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    pub total_tests: u32,
    pub passed_tests: u32,
    pub failed_tests: u32,
    pub skipped_tests: u32,
    pub success_rate: f64,
    pub average_response_time_ms: f64,
    pub performance_violations: u32,
    pub security_vulnerabilities: u32,
    pub service_issues: u32,
}

/// Integration test runner
pub struct IntegrationTestRunner {
    config: IntegrationTestConfig,
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    rate_limiter: Arc<RateLimiter>,
    trading_guard: Arc<TradingGuard>,
    model_manager: Arc<ModelManager>,
    trading_service: Arc<AutomatedTradingServiceImpl>,
}

impl IntegrationTestRunner {
    /// Create new integration test runner
    pub async fn new(config: IntegrationTestConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize core dependencies
        let state = Arc::new(AppState::new());
        let auth_service = Arc::new(AuthService::new(state.clone()));
        let audit_logger = Arc::new(AuditLogger::new(state.clone()));
        let rate_limiter = Arc::new(RateLimiter::new());
        let trading_guard = Arc::new(TradingGuard::new(
            auth_service.clone(),
            audit_logger.clone(),
            rate_limiter.clone(),
        ));

        // Initialize ML model manager
        let ml_config = fo3_wallet_api::ml::MLConfig::default();
        let model_manager = Arc::new(ModelManager::new(ml_config));

        // Initialize trading service
        let trading_service = Arc::new(AutomatedTradingServiceImpl::new(
            auth_service.clone(),
            audit_logger.clone(),
            trading_guard.clone(),
            model_manager.clone(),
        ));

        Ok(Self {
            config,
            state,
            auth_service,
            audit_logger,
            rate_limiter,
            trading_guard,
            model_manager,
            trading_service,
        })
    }

    /// Run comprehensive integration tests
    pub async fn run_comprehensive_tests(&mut self) -> Result<ComprehensiveTestResults, Box<dyn std::error::Error>> {
        let test_run_id = uuid::Uuid::new_v4().to_string();
        let started_at = Utc::now();
        let start_time = Instant::now();

        println!("🚀 Starting Phase 3 Integration Testing & Quality Assurance");
        println!("📋 Test Run ID: {}", test_run_id);
        println!("⏰ Started at: {}", started_at.format("%Y-%m-%d %H:%M:%S UTC"));

        // Initialize test frameworks
        let mut e2e_framework = E2ETestFramework::new().await?;
        let mut performance_validator = PerformanceValidator::new(
            self.config.performance_config.clone(),
            self.model_manager.clone(),
            self.trading_service.clone(),
        );
        let mut security_validator = SecurityValidator::new(
            self.config.security_config.clone(),
            self.auth_service.clone(),
            self.audit_logger.clone(),
            self.rate_limiter.clone(),
            self.trading_guard.clone(),
            self.state.clone(),
        );
        let mut service_validator = ServiceRegistrationValidator::new(self.state.clone());

        // Initialize service validator client
        service_validator.initialize_client(&self.config.server_address).await?;

        let mut results = ComprehensiveTestResults {
            test_run_id: test_run_id.clone(),
            started_at,
            completed_at: Utc::now(), // Will be updated
            total_duration: Duration::from_secs(0), // Will be updated
            overall_success: false, // Will be calculated
            e2e_results: vec![],
            performance_results: vec![],
            security_results: vec![],
            service_registration_results: vec![],
            validation_report: ValidationReport {
                total_services: 0,
                registered_services: 0,
                healthy_services: 0,
                total_endpoints: 0,
                successful_endpoints: 0,
                overall_success: false,
                issues: vec![],
                recommendations: vec![],
            },
            summary: TestSummary {
                total_tests: 0,
                passed_tests: 0,
                failed_tests: 0,
                skipped_tests: 0,
                success_rate: 0.0,
                average_response_time_ms: 0.0,
                performance_violations: 0,
                security_vulnerabilities: 0,
                service_issues: 0,
            },
            recommendations: vec![],
        };

        // 1. Service Registration Validation
        println!("\n📡 Phase 1: Service Registration Validation");
        results.service_registration_results = service_validator.validate_all_services().await?;
        results.validation_report = service_validator.generate_validation_report(&results.service_registration_results);
        
        self.print_service_validation_summary(&results.service_registration_results);

        // 2. End-to-End Testing
        println!("\n🔄 Phase 2: End-to-End Testing");
        results.e2e_results = e2e_framework.run_comprehensive_test_suite().await?;
        
        self.print_e2e_summary(&results.e2e_results);

        // 3. Performance Validation
        println!("\n⚡ Phase 3: Performance Validation");
        results.performance_results = performance_validator.run_performance_validation().await?;
        
        self.print_performance_summary(&results.performance_results);

        // 4. Security Validation
        println!("\n🔒 Phase 4: Security Validation");
        results.security_results = security_validator.run_security_validation().await?;
        
        self.print_security_summary(&results.security_results);

        // Calculate final results
        let completed_at = Utc::now();
        let total_duration = start_time.elapsed();

        results.completed_at = completed_at;
        results.total_duration = total_duration;
        results.summary = self.calculate_test_summary(&results);
        results.overall_success = self.calculate_overall_success(&results);
        results.recommendations = self.generate_comprehensive_recommendations(&results);

        // Generate reports if configured
        if self.config.generate_reports {
            self.generate_test_reports(&results).await?;
        }

        // Print final summary
        self.print_final_summary(&results);

        Ok(results)
    }

    /// Print service validation summary
    fn print_service_validation_summary(&self, results: &[ServiceRegistrationResult]) {
        let total = results.len();
        let registered = results.iter()
            .filter(|r| matches!(r.registration_status, service_registration_validation::RegistrationStatus::Registered))
            .count();
        let healthy = results.iter()
            .filter(|r| matches!(r.health_check_status, service_registration_validation::HealthStatus::Healthy))
            .count();

        println!("  ✅ Services Registered: {}/{}", registered, total);
        println!("  💚 Services Healthy: {}/{}", healthy, total);
        
        for result in results {
            let status_icon = match (&result.registration_status, &result.health_check_status) {
                (service_registration_validation::RegistrationStatus::Registered, service_registration_validation::HealthStatus::Healthy) => "✅",
                _ => "❌",
            };
            println!("    {} {}", status_icon, result.service_name);
        }
    }

    /// Print E2E summary
    fn print_e2e_summary(&self, results: &[TestScenarioResult]) {
        let total = results.len();
        let passed = results.iter().filter(|r| r.success).count();
        
        println!("  ✅ E2E Tests Passed: {}/{}", passed, total);
        
        for result in results {
            let status_icon = if result.success { "✅" } else { "❌" };
            println!("    {} {} ({}ms)", status_icon, result.scenario_name, result.duration.as_millis());
        }
    }

    /// Print performance summary
    fn print_performance_summary(&self, results: &[PerformanceTestResult]) {
        let total = results.len();
        let passed = results.iter().filter(|r| r.overall_success).count();
        
        println!("  ⚡ Performance Tests Passed: {}/{}", passed, total);
        
        for result in results {
            let status_icon = if result.overall_success { "✅" } else { "❌" };
            let violations = result.violations.len();
            println!("    {} {} ({} violations)", status_icon, result.test_name, violations);
        }
    }

    /// Print security summary
    fn print_security_summary(&self, results: &[SecurityTestResult]) {
        let total = results.len();
        let passed = results.iter().filter(|r| r.success).count();
        
        println!("  🔒 Security Tests Passed: {}/{}", passed, total);
        
        for result in results {
            let status_icon = if result.success { "✅" } else { "❌" };
            let vulnerabilities = result.vulnerabilities.len();
            println!("    {} {} ({} vulnerabilities)", status_icon, result.test_name, vulnerabilities);
        }
    }

    /// Calculate test summary
    fn calculate_test_summary(&self, results: &ComprehensiveTestResults) -> TestSummary {
        let e2e_total = results.e2e_results.len() as u32;
        let e2e_passed = results.e2e_results.iter().filter(|r| r.success).count() as u32;

        let perf_total = results.performance_results.len() as u32;
        let perf_passed = results.performance_results.iter().filter(|r| r.overall_success).count() as u32;

        let sec_total = results.security_results.len() as u32;
        let sec_passed = results.security_results.iter().filter(|r| r.success).count() as u32;

        let svc_total = results.service_registration_results.len() as u32;
        let svc_passed = results.service_registration_results.iter()
            .filter(|r| matches!(r.registration_status, service_registration_validation::RegistrationStatus::Registered) &&
                       matches!(r.health_check_status, service_registration_validation::HealthStatus::Healthy))
            .count() as u32;

        let total_tests = e2e_total + perf_total + sec_total + svc_total;
        let passed_tests = e2e_passed + perf_passed + sec_passed + svc_passed;
        let failed_tests = total_tests - passed_tests;

        let success_rate = if total_tests > 0 {
            (passed_tests as f64 / total_tests as f64) * 100.0
        } else {
            0.0
        };

        let average_response_time_ms = results.e2e_results.iter()
            .map(|r| r.duration.as_millis() as f64)
            .sum::<f64>() / results.e2e_results.len().max(1) as f64;

        let performance_violations = results.performance_results.iter()
            .map(|r| r.violations.len() as u32)
            .sum();

        let security_vulnerabilities = results.security_results.iter()
            .map(|r| r.vulnerabilities.len() as u32)
            .sum();

        let service_issues = results.service_registration_results.iter()
            .filter(|r| !matches!(r.registration_status, service_registration_validation::RegistrationStatus::Registered) ||
                       !matches!(r.health_check_status, service_registration_validation::HealthStatus::Healthy))
            .count() as u32;

        TestSummary {
            total_tests,
            passed_tests,
            failed_tests,
            skipped_tests: 0,
            success_rate,
            average_response_time_ms,
            performance_violations,
            security_vulnerabilities,
            service_issues,
        }
    }

    /// Calculate overall success
    fn calculate_overall_success(&self, results: &ComprehensiveTestResults) -> bool {
        results.summary.success_rate >= 95.0 && // 95% success rate requirement
        results.summary.performance_violations == 0 && // No performance violations
        results.summary.security_vulnerabilities == 0 && // No security vulnerabilities
        results.summary.service_issues == 0 // No service issues
    }

    /// Generate comprehensive recommendations
    fn generate_comprehensive_recommendations(&self, results: &ComprehensiveTestResults) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Performance recommendations
        for perf_result in &results.performance_results {
            recommendations.extend(perf_result.recommendations.clone());
        }

        // Security recommendations
        for sec_result in &results.security_results {
            recommendations.extend(sec_result.recommendations.clone());
        }

        // Service recommendations
        recommendations.extend(results.validation_report.recommendations.clone());

        // General recommendations based on overall results
        if results.summary.success_rate < 95.0 {
            recommendations.push("Improve test coverage and fix failing tests".to_string());
        }

        if results.summary.average_response_time_ms > 200.0 {
            recommendations.push("Optimize response times to meet <200ms requirement".to_string());
        }

        if results.summary.performance_violations > 0 {
            recommendations.push("Address performance violations before production deployment".to_string());
        }

        if results.summary.security_vulnerabilities > 0 {
            recommendations.push("Fix security vulnerabilities before production deployment".to_string());
        }

        recommendations.dedup();
        recommendations
    }

    /// Generate test reports
    async fn generate_test_reports(&self, results: &ComprehensiveTestResults) -> Result<(), Box<dyn std::error::Error>> {
        // Create output directory
        tokio::fs::create_dir_all(&self.config.report_output_dir).await?;

        // Generate JSON report
        let json_report = serde_json::to_string_pretty(results)?;
        let json_path = format!("{}/integration_test_report_{}.json", 
            self.config.report_output_dir, results.test_run_id);
        tokio::fs::write(json_path, json_report).await?;

        // Generate HTML report (simplified)
        let html_report = self.generate_html_report(results);
        let html_path = format!("{}/integration_test_report_{}.html", 
            self.config.report_output_dir, results.test_run_id);
        tokio::fs::write(html_path, html_report).await?;

        println!("📊 Test reports generated in: {}", self.config.report_output_dir);

        Ok(())
    }

    /// Generate HTML report
    fn generate_html_report(&self, results: &ComprehensiveTestResults) -> String {
        format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>FO3 Wallet Core - Integration Test Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background-color: #f0f0f0; padding: 20px; border-radius: 5px; }}
        .summary {{ margin: 20px 0; }}
        .success {{ color: green; }}
        .failure {{ color: red; }}
        .warning {{ color: orange; }}
        table {{ border-collapse: collapse; width: 100%; margin: 20px 0; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #f2f2f2; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>FO3 Wallet Core - Integration Test Report</h1>
        <p><strong>Test Run ID:</strong> {}</p>
        <p><strong>Started:</strong> {}</p>
        <p><strong>Completed:</strong> {}</p>
        <p><strong>Duration:</strong> {:.2}s</p>
    </div>
    
    <div class="summary">
        <h2>Test Summary</h2>
        <p class="{}"><strong>Overall Success:</strong> {}</p>
        <p><strong>Success Rate:</strong> {:.1}%</p>
        <p><strong>Total Tests:</strong> {}</p>
        <p><strong>Passed:</strong> {}</p>
        <p><strong>Failed:</strong> {}</p>
        <p><strong>Average Response Time:</strong> {:.1}ms</p>
    </div>

    <div class="recommendations">
        <h2>Recommendations</h2>
        <ul>
        {}
        </ul>
    </div>
</body>
</html>
        "#,
            results.test_run_id,
            results.started_at.format("%Y-%m-%d %H:%M:%S UTC"),
            results.completed_at.format("%Y-%m-%d %H:%M:%S UTC"),
            results.total_duration.as_secs_f64(),
            if results.overall_success { "success" } else { "failure" },
            results.overall_success,
            results.summary.success_rate,
            results.summary.total_tests,
            results.summary.passed_tests,
            results.summary.failed_tests,
            results.summary.average_response_time_ms,
            results.recommendations.iter()
                .map(|r| format!("<li>{}</li>", r))
                .collect::<Vec<_>>()
                .join("\n        ")
        )
    }

    /// Print final summary
    fn print_final_summary(&self, results: &ComprehensiveTestResults) {
        println!("\n" + "=".repeat(80).as_str());
        println!("🎯 PHASE 3 INTEGRATION TESTING & QUALITY ASSURANCE COMPLETE");
        println!("=".repeat(80));
        
        let status_icon = if results.overall_success { "✅" } else { "❌" };
        let status_text = if results.overall_success { "PASSED" } else { "FAILED" };
        
        println!("\n{} OVERALL STATUS: {}", status_icon, status_text);
        println!("📊 Success Rate: {:.1}%", results.summary.success_rate);
        println!("⏱️  Total Duration: {:.2}s", results.total_duration.as_secs_f64());
        println!("🧪 Total Tests: {}", results.summary.total_tests);
        println!("✅ Passed: {}", results.summary.passed_tests);
        println!("❌ Failed: {}", results.summary.failed_tests);
        println!("⚡ Avg Response Time: {:.1}ms", results.summary.average_response_time_ms);
        
        if results.summary.performance_violations > 0 {
            println!("⚠️  Performance Violations: {}", results.summary.performance_violations);
        }
        
        if results.summary.security_vulnerabilities > 0 {
            println!("🔒 Security Vulnerabilities: {}", results.summary.security_vulnerabilities);
        }
        
        if results.summary.service_issues > 0 {
            println!("📡 Service Issues: {}", results.summary.service_issues);
        }

        if !results.recommendations.is_empty() {
            println!("\n💡 KEY RECOMMENDATIONS:");
            for (i, rec) in results.recommendations.iter().take(5).enumerate() {
                println!("  {}. {}", i + 1, rec);
            }
        }

        if results.overall_success {
            println!("\n🚀 READY FOR PRODUCTION DEPLOYMENT!");
        } else {
            println!("\n⚠️  ISSUES MUST BE RESOLVED BEFORE PRODUCTION DEPLOYMENT");
        }
        
        println!("\n" + "=".repeat(80).as_str());
    }
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self {
            server_address: "http://localhost:50051".to_string(),
            test_timeout_seconds: 300,
            parallel_execution: false,
            generate_reports: true,
            report_output_dir: "./test_reports".to_string(),
            performance_config: PerformanceTestConfig::default(),
            security_config: SecurityTestConfig::default(),
        }
    }
}
